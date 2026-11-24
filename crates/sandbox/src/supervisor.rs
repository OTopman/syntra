use crate::capabilities::{CapabilityEvidenceRecord, CleanupVerification};
use crate::config::StreamBudgetConfig;
use crate::error::SandboxError;
use crate::traits::IsolatedChild;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, OwnedFd};

/// Outcome returned by `ProcessSupervisor::supervise`.
#[derive(Debug)]
pub struct SupervisedExecution {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub timed_out: bool,
    pub stream_overflow: Option<(&'static str, usize)>,
    pub telemetry_error: Option<SandboxError>,
    pub evidence: Vec<CapabilityEvidenceRecord>,
    pub cleanup: CleanupVerification,
}

/// Platform-agnostic process lifecycle and bounded non-blocking I/O supervisor.
pub struct ProcessSupervisor;

impl ProcessSupervisor {
    /// Supervises an active `IsolatedChild`, collecting bounded stdout/stderr via
    /// non-blocking `poll()` without cross-thread descriptor close races, enforcing
    /// wall-clock deadlines, and executing deterministic teardown and cleanup.
    pub fn supervise(
        child: &mut dyn IsolatedChild,
        streams: &StreamBudgetConfig,
        wall_clock_timeout: Duration,
    ) -> Result<SupervisedExecution, SandboxError> {
        #[cfg(unix)]
        {
            Self::supervise_unix(child, streams, wall_clock_timeout)
        }
        #[cfg(not(unix))]
        {
            Self::supervise_generic(child, streams, wall_clock_timeout)
        }
    }

    #[cfg(unix)]
    fn supervise_unix(
        child: &mut dyn IsolatedChild,
        streams: &StreamBudgetConfig,
        wall_clock_timeout: Duration,
    ) -> Result<SupervisedExecution, SandboxError> {
        let t0 = Instant::now();
        let wall_deadline = t0 + wall_clock_timeout;

        let mut stdout_fd = child.take_stdout_fd();
        let mut stderr_fd = child.take_stderr_fd();

        if let Some(ref fd) = stdout_fd {
            set_nonblocking(fd.as_raw_fd())?;
        }
        if let Some(ref fd) = stderr_fd {
            set_nonblocking(fd.as_raw_fd())?;
        }

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        let mut stdout_overflow = false;
        let mut stderr_overflow = false;
        let mut stream_overflow: Option<(&'static str, usize)> = None;
        let mut timed_out = false;
        let mut exit_code: Option<i32> = None;

        loop {
            let now = Instant::now();
            if now >= wall_deadline {
                timed_out = true;
                child.kill_tree()?;
                break;
            }

            let remaining = wall_deadline.saturating_duration_since(now);
            let slice_ms = remaining.as_millis().min(20) as libc::c_int;
            poll_and_drain_once(
                &mut stdout_fd,
                &mut stderr_fd,
                &mut stdout_buf,
                &mut stderr_buf,
                streams,
                &mut stdout_overflow,
                &mut stderr_overflow,
                slice_ms,
            )?;

            if stdout_overflow && stream_overflow.is_none() {
                stream_overflow = Some(("stdout", streams.stdout_limit_bytes));
                if streams.terminate_on_overflow {
                    child.kill_tree()?;
                    break;
                }
            }
            if stderr_overflow && stream_overflow.is_none() {
                stream_overflow = Some(("stderr", streams.stderr_limit_bytes));
                if streams.terminate_on_overflow {
                    child.kill_tree()?;
                    break;
                }
            }

            match child.try_wait()? {
                Some(code) => {
                    exit_code = Some(code);
                    break;
                }
                None => {
                    // Wait only occurred if both pipes closed early; keep CPU idle briefly.
                    if stdout_fd.is_none() && stderr_fd.is_none() {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
            }
        }

        // Bounded post-exit / post-kill drain window (default 500 ms).
        let drain_deadline = Instant::now() + streams.drain_timeout;
        while (stdout_fd.is_some() || stderr_fd.is_some()) && Instant::now() < drain_deadline {
            let rem = drain_deadline.saturating_duration_since(Instant::now());
            let slice_ms = rem.as_millis().min(25) as libc::c_int;
            poll_and_drain_once(
                &mut stdout_fd,
                &mut stderr_fd,
                &mut stdout_buf,
                &mut stderr_buf,
                streams,
                &mut stdout_overflow,
                &mut stderr_overflow,
                slice_ms,
            )?;
            if stdout_overflow && stream_overflow.is_none() {
                stream_overflow = Some(("stdout", streams.stdout_limit_bytes));
                if streams.terminate_on_overflow {
                    let _ = child.kill_tree();
                }
            }
            if stderr_overflow && stream_overflow.is_none() {
                stream_overflow = Some(("stderr", streams.stderr_limit_bytes));
                if streams.terminate_on_overflow {
                    let _ = child.kill_tree();
                }
            }
        }

        // Same thread explicitly drops OwnedFds at exit of drain loop — zero cross-thread close race.
        drop(stdout_fd);
        drop(stderr_fd);

        // If the process timed out or overflowed and wasn't reaped yet, ensure termination + cleanup.
        if exit_code.is_none() {
            if let Ok(Some(code)) = child.try_wait() {
                exit_code = Some(code);
            } else {
                let _ = child.kill_tree();
                if let Ok(Some(code)) = child.try_wait() {
                    exit_code = Some(code);
                }
            }
        }

        let duration = t0.elapsed();
        let telemetry_error = child.post_run_telemetry();
        let evidence = child.capability_evidence();
        let cleanup = child.verify_and_cleanup()?;

        Ok(SupervisedExecution {
            stdout: String::from_utf8_lossy(&stdout_buf).into_owned(),
            stderr: String::from_utf8_lossy(&stderr_buf).into_owned(),
            exit_code,
            duration,
            timed_out,
            stream_overflow,
            telemetry_error,
            evidence,
            cleanup,
        })
    }

    #[cfg(not(unix))]
    fn supervise_generic(
        child: &mut dyn IsolatedChild,
        streams: &StreamBudgetConfig,
        wall_clock_timeout: Duration,
    ) -> Result<SupervisedExecution, SandboxError> {
        use std::sync::mpsc;

        let t0 = Instant::now();
        let wall_deadline = t0 + wall_clock_timeout;

        let stdout_reader = child.take_stdout_reader();
        let stderr_reader = child.take_stderr_reader();

        let stdout_limit = streams.stdout_limit_bytes;
        let stderr_limit = streams.stderr_limit_bytes;

        let (tx_out, rx_out) = mpsc::channel();
        if let Some(mut r) = stdout_reader {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let mut overflow = false;
                let mut tmp = [0u8; 4096];
                while let Ok(n) = r.read(&mut tmp) {
                    if n == 0 {
                        break;
                    }
                    let rem = stdout_limit.saturating_sub(buf.len());
                    let take = n.min(rem);
                    buf.extend_from_slice(&tmp[..take]);
                    if n > rem {
                        overflow = true;
                        break;
                    }
                }
                let _ = tx_out.send((buf, overflow));
            });
        } else {
            let _ = tx_out.send((Vec::new(), false));
        }

        let (tx_err, rx_err) = mpsc::channel();
        if let Some(mut r) = stderr_reader {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let mut overflow = false;
                let mut tmp = [0u8; 4096];
                while let Ok(n) = r.read(&mut tmp) {
                    if n == 0 {
                        break;
                    }
                    let rem = stderr_limit.saturating_sub(buf.len());
                    let take = n.min(rem);
                    buf.extend_from_slice(&tmp[..take]);
                    if n > rem {
                        overflow = true;
                        break;
                    }
                }
                let _ = tx_err.send((buf, overflow));
            });
        } else {
            let _ = tx_err.send((Vec::new(), false));
        }

        let mut timed_out = false;
        let mut exit_code = None;

        loop {
            if Instant::now() >= wall_deadline {
                timed_out = true;
                child.kill_tree()?;
                break;
            }
            if let Some(code) = child.try_wait()? {
                exit_code = Some(code);
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        let (stdout_buf, out_ov) = rx_out
            .recv_timeout(streams.drain_timeout)
            .unwrap_or_else(|_| (Vec::new(), false));
        let (stderr_buf, err_ov) = rx_err
            .recv_timeout(streams.drain_timeout)
            .unwrap_or_else(|_| (Vec::new(), false));

        let stream_overflow = if out_ov {
            Some(("stdout", stdout_limit))
        } else if err_ov {
            Some(("stderr", stderr_limit))
        } else {
            None
        };

        let duration = t0.elapsed();
        let telemetry_error = child.post_run_telemetry();
        let evidence = child.capability_evidence();
        let cleanup = child.verify_and_cleanup()?;

        Ok(SupervisedExecution {
            stdout: String::from_utf8_lossy(&stdout_buf).into_owned(),
            stderr: String::from_utf8_lossy(&stderr_buf).into_owned(),
            exit_code,
            duration,
            timed_out,
            stream_overflow,
            telemetry_error,
            evidence,
            cleanup,
        })
    }
}

#[cfg(unix)]
fn set_nonblocking(fd: libc::c_int) -> Result<(), SandboxError> {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 {
            return Err(SandboxError::IoFailure(format!(
                "fcntl(F_GETFL) failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        if libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(SandboxError::IoFailure(format!(
                "fcntl(F_SETFL, O_NONBLOCK) failed: {}",
                std::io::Error::last_os_error()
            )));
        }
    }
    Ok(())
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn poll_and_drain_once(
    stdout_fd: &mut Option<OwnedFd>,
    stderr_fd: &mut Option<OwnedFd>,
    stdout_buf: &mut Vec<u8>,
    stderr_buf: &mut Vec<u8>,
    streams: &StreamBudgetConfig,
    stdout_overflow: &mut bool,
    stderr_overflow: &mut bool,
    timeout_ms: libc::c_int,
) -> Result<(), SandboxError> {
    let mut fds = Vec::with_capacity(2);
    let mut is_stdout = [false; 2];

    if let Some(ref fd) = stdout_fd {
        is_stdout[fds.len()] = true;
        fds.push(libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        });
    }
    if let Some(ref fd) = stderr_fd {
        is_stdout[fds.len()] = false;
        fds.push(libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        });
    }

    if fds.is_empty() {
        return Ok(());
    }

    let rc = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout_ms) };
    if rc < 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::Interrupted {
            return Ok(());
        }
        return Err(SandboxError::IoFailure(format!("poll() failed: {err}")));
    }

    for (idx, pfd) in fds.iter().enumerate() {
        if pfd.revents == 0 {
            continue;
        }
        if is_stdout[idx] {
            let eof_or_ov = drain_nonblocking_fd(
                pfd.fd,
                stdout_buf,
                streams.stdout_limit_bytes,
                stdout_overflow,
            )?;
            if eof_or_ov || (pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL)) != 0 {
                // Close only from polling thread after read drains remaining buffered data.
                *stdout_fd = None;
            }
        } else {
            let eof_or_ov = drain_nonblocking_fd(
                pfd.fd,
                stderr_buf,
                streams.stderr_limit_bytes,
                stderr_overflow,
            )?;
            if eof_or_ov || (pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL)) != 0 {
                *stderr_fd = None;
            }
        }
    }

    Ok(())
}

#[cfg(unix)]
fn drain_nonblocking_fd(
    fd: libc::c_int,
    out: &mut Vec<u8>,
    limit_bytes: usize,
    overflow_flag: &mut bool,
) -> Result<bool, SandboxError> {
    let mut chunk = [0u8; 4096];
    loop {
        let rem = limit_bytes.saturating_sub(out.len());
        let max_read = if rem == 0 { 1 } else { chunk.len().min(rem) };
        let n = unsafe { libc::read(fd, chunk.as_mut_ptr() as *mut libc::c_void, max_read) };
        if n > 0 {
            let n = n as usize;
            if rem == 0 {
                *overflow_flag = true;
                return Ok(true);
            }
            out.extend_from_slice(&chunk[..n]);
        } else if n == 0 {
            return Ok(true); // EOF
        } else {
            let err = std::io::Error::last_os_error();
            match err.raw_os_error() {
                Some(code) if code == libc::EAGAIN || code == libc::EWOULDBLOCK => {
                    return Ok(false);
                }
                Some(code) if code == libc::EINTR => continue,
                _ => return Err(SandboxError::IoFailure(format!("read(pipe) failed: {err}"))),
            }
        }
    }
}
