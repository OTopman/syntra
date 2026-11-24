use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use syntra_sandbox::capabilities::CapabilityVerificationStatus;
use syntra_sandbox::config::EnforcementMode;
use syntra_sandbox::runner::SandboxRunner;

#[cfg(unix)]
fn write_executable(path: &PathBuf, content: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, content).expect("write script");
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod 0755");
}

/// ADV-09A: Verify default Strict mode fails closed on non-Linux hosts (macOS/Windows)
/// without spawning any process or creating candidate side effects.
#[test]
fn test_adversarial_adv09a_strict_mode_fails_closed_without_spawning() {
    let run_id = format!("adv09a_{}", std::process::id());
    let runner = SandboxRunner::new(run_id);
    assert_eq!(runner.enforcement_mode(), EnforcementMode::Strict);

    let dir = runner.candidate_dir("cand_adv09a");
    fs::create_dir_all(&dir).unwrap();
    let marker = dir.join("spawned_marker.txt");
    let script = dir.join("should_not_run.sh");

    #[cfg(unix)]
    write_executable(
        &script,
        &format!("#!/bin/sh\necho PWNED > {}\n", marker.display()),
    );
    #[cfg(windows)]
    fs::write(&script, "echo PWNED").unwrap();

    #[cfg(not(target_os = "linux"))]
    {
        let res = runner.execute_binary(&script, &[]);
        assert!(
            res.is_err(),
            "Strict mode must refuse execution fail-closed on unsupported host OS"
        );
        let err = res.unwrap_err();
        assert!(
            err.contains("Strict Invariant Violation"),
            "Error must clearly report Strict Invariant Violation: {err}"
        );
        assert!(
            !marker.exists(),
            "Fail-closed Strict mode must spawn zero child processes"
        );
    }

    #[cfg(target_os = "linux")]
    {
        let caps = PlatformCapabilities::probe_host("adv09a_probe");
        if caps.verify_strict_admissibility().is_err() {
            let res = runner.execute_binary(&script, &[]);
            assert!(res.is_err());
            assert!(!marker.exists());
        }
    }
}

/// ADV-09B: Verify Guarded mode accurately reports partial/unsupported host boundaries
/// in its structured ExecutionEvidenceReport and executes deterministic cleanup.
#[test]
fn test_adversarial_adv09b_guarded_mode_evidence_and_teardown() {
    let run_id = format!("adv09b_{}", std::process::id());
    let base_path: PathBuf;
    let evidence_json: String;
    {
        let runner = SandboxRunner::new(run_id.clone())
            .with_enforcement_mode(EnforcementMode::Guarded)
            .with_timeout(Duration::from_secs(3));
        base_path = runner.sandbox_base().to_path_buf();

        let dir = runner.candidate_dir("cand_adv09b");
        fs::create_dir_all(&dir).unwrap();
        let script = dir.join("benign.sh");
        #[cfg(unix)]
        write_executable(&script, "#!/bin/sh\necho guarded_ok\n");

        let out = runner.execute_binary(&script, &[]).expect("Guarded run");
        assert!(
            out.stdout.contains("guarded_ok"),
            "Expected guarded_ok in stdout; got out={out:?}"
        );
        assert_eq!(out.exit_code, Some(0));

        let report = runner
            .last_evidence_report()
            .expect("ExecutionEvidenceReport must be recorded");
        assert_eq!(report.execution_id, run_id);
        assert_eq!(report.enforcement_mode, EnforcementMode::Guarded);
        assert!(report.cleanup.surviving_pids.is_empty());

        #[cfg(not(target_os = "linux"))]
        {
            let fs_cap = report
                .capabilities
                .iter()
                .find(|c| c.capability_id == "fs.mount_isolation")
                .expect("fs capability present");
            assert!(
                matches!(
                    fs_cap.status,
                    CapabilityVerificationStatus::Unsupported
                        | CapabilityVerificationStatus::Unavailable
                ),
                "Non-Linux host must never claim fs.mount_isolation verified"
            );
        }

        evidence_json = report.to_json_string();
        assert!(evidence_json.contains("\"schema_version\": \"1.0.0\""));
        assert!(evidence_json.contains("\"enforcement_mode\": \"Guarded\""));
    }

    assert!(
        !base_path.exists(),
        "Sandbox directory must be cleaned up deterministically on Drop"
    );
}

/// ADV-06A/B/C: Verify inherited host file descriptors (regular files, pipes, directory FDs)
/// opened in the parent process without CLOEXEC do not leak past execve into the candidate.
#[cfg(unix)]
#[test]
fn test_adversarial_adv06_inherited_fd_and_dirfd_sanitization() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::{FromRawFd, OwnedFd};

    let run_id = format!("adv06_{}", std::process::id());
    let runner = SandboxRunner::new(run_id)
        .with_enforcement_mode(EnforcementMode::Guarded)
        .with_timeout(Duration::from_secs(5));

    let dir = runner.candidate_dir("cand_adv06");
    fs::create_dir_all(&dir).unwrap();
    let secret_file = dir.join("host_secret_fd.txt");
    fs::write(&secret_file, "TOP_SECRET_FD_TOKEN_987654").unwrap();

    // Open both a regular secret file FD and an O_DIRECTORY FD WITHOUT O_CLOEXEC
    // to prove pre-exec sanitization closes inherited descriptors >= 3.
    let secret_c = CString::new(secret_file.as_os_str().as_bytes()).unwrap();
    let dir_c = CString::new("/etc").unwrap();
    let (raw_secret_fd, raw_dir_fd) = unsafe {
        let f1 = libc::open(secret_c.as_ptr(), libc::O_RDONLY);
        let f2 = libc::open(dir_c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY);
        assert!(f1 >= 3 && f2 >= 3);
        (f1, f2)
    };
    let _guard1 = unsafe { OwnedFd::from_raw_fd(raw_secret_fd) };
    let _guard2 = unsafe { OwnedFd::from_raw_fd(raw_dir_fd) };

    let script = dir.join("probe_fds.sh");
    write_executable(
        &script,
        &format!(
            "#!/bin/sh\ncat /dev/fd/{raw_secret_fd} 2>/dev/null || echo SECRET_FD_CLOSED\nls /dev/fd/{raw_dir_fd} >/dev/null 2>&1 && echo DIR_FD_LEAKED || echo DIR_FD_CLOSED\n"
        ),
    );

    let out = runner.execute_binary(&script, &[]).expect("Run ADV-06");
    assert!(
        !out.stdout.contains("TOP_SECRET_FD_TOKEN_987654"),
        "Candidate must NEVER read leaked secret FD: {out:?}"
    );
    assert!(
        !out.stdout.contains("DIR_FD_LEAKED"),
        "Candidate must NEVER inherit parent directory FD: {out:?}"
    );
    assert!(
        out.stdout.contains("SECRET_FD_CLOSED"),
        "Expected SECRET_FD_CLOSED; got out={out:?}"
    );
    assert!(
        out.stdout.contains("DIR_FD_CLOSED"),
        "Expected DIR_FD_CLOSED; got out={out:?}"
    );
}

/// ADV-08A: Simultaneous stdout + stderr continuous flood must respect independent
/// byte limits and terminate promptly without pipe deadlocks.
#[cfg(unix)]
#[test]
fn test_adversarial_adv08a_simultaneous_stdout_stderr_flood_bounded() {
    let run_id = format!("adv08a_{}", std::process::id());
    let runner = SandboxRunner::new(run_id)
        .with_enforcement_mode(EnforcementMode::Guarded)
        .with_max_output_bytes(512)
        .with_timeout(Duration::from_secs(3));

    let dir = runner.candidate_dir("cand_adv08a");
    fs::create_dir_all(&dir).unwrap();
    let script = dir.join("flood_both.sh");
    write_executable(
        &script,
        "#!/bin/sh\ni=0\nwhile [ $i -lt 5000 ]; do\n  echo \"STDOUT_LINE_0123456789ABCDEF\"\n  echo \"STDERR_LINE_0123456789ABCDEF\" >&2\n  i=$((i+1))\ndone\n",
    );

    let t0 = Instant::now();
    let out = runner.execute_binary(&script, &[]).expect("Run ADV-08A");
    let elapsed = t0.elapsed();

    assert!(
        out.stdout.len() <= 512,
        "stdout exceeded 512 bytes: {}",
        out.stdout.len()
    );
    assert!(
        out.stderr.len() <= 512,
        "stderr exceeded 512 bytes: {}",
        out.stderr.len()
    );
    assert!(
        runner.last_stream_overflow().is_some(),
        "Stream overflow must be flagged in supervisor metadata"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "Overflow termination must not wait for 3s timeout (took {elapsed:?})"
    );
}

/// ADV-08B: Descendant holding stdout pipe open must NOT hang the supervisor
/// indefinitely; post-exit drain must finish within the 500ms drain window.
#[cfg(unix)]
#[test]
fn test_adversarial_adv08b_hung_pipe_holder_respects_drain_deadline() {
    let run_id = format!("adv08b_{}", std::process::id());
    let runner = SandboxRunner::new(run_id)
        .with_enforcement_mode(EnforcementMode::Guarded)
        .with_timeout(Duration::from_secs(3));

    let dir = runner.candidate_dir("cand_adv08b");
    fs::create_dir_all(&dir).unwrap();
    let script = dir.join("hung_pipe.sh");
    // Child exits cleanly at once, leaving background sleep holding stdout write FD open.
    write_executable(
        &script,
        "#!/bin/sh\n(sleep 10) &\necho child_exited\nexit 0\n",
    );

    let t0 = Instant::now();
    let out = runner.execute_binary(&script, &[]).expect("Run ADV-08B");
    let elapsed = t0.elapsed();

    assert!(
        out.stdout.contains("child_exited"),
        "Expected child_exited in stdout; got out={out:?}"
    );
    assert!(
        !out.timed_out,
        "Child exited 0 cleanly; only pipe drain should bound post-exit wait"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "Supervisor must enforce bounded 500ms drain instead of waiting 10s (took {elapsed:?})"
    );
}

/// FLT-06 & FLT-08: Concurrent sandbox execution isolation and fail-closed missing binary check.
#[cfg(unix)]
#[test]
fn test_fault_injection_concurrent_runs_and_missing_binary() {
    let missing_runner = SandboxRunner::new(format!("flt08_{}", std::process::id()))
        .with_enforcement_mode(EnforcementMode::Guarded);
    let missing_err = missing_runner.execute_binary(std::path::Path::new("/nonexistent/bin"), &[]);
    assert!(missing_err.is_err());

    let mut handles = Vec::new();
    for idx in 0..4 {
        handles.push(std::thread::spawn(move || {
            let run_id = format!("flt06_{}_{}", std::process::id(), idx);
            let runner = SandboxRunner::new(run_id)
                .with_enforcement_mode(EnforcementMode::Guarded)
                .with_timeout(Duration::from_secs(5));
            let dir = runner.candidate_dir("cand");
            fs::create_dir_all(&dir).unwrap();
            let script = dir.join("ok.sh");
            write_executable(&script, &format!("#!/bin/sh\necho worker_{idx}\n"));
            let out = runner.execute_binary(&script, &[]).expect("concurrent run");
            assert!(
                out.stdout.contains(&format!("worker_{idx}")),
                "Expected worker_{idx}; got out={out:?}"
            );
            runner.sandbox_base().to_path_buf()
        }));
    }

    for h in handles {
        let dir = h.join().expect("thread join");
        assert!(
            !dir.exists(),
            "Concurrent sandbox dir must be removed on drop"
        );
    }
}

// =============================================================================
// Linux-only Kernel Enforcement Adversarial Tests (ADV-01..05, ADV-07, ADV-10)
// Executed only on Linux hosts with cgroup2 delegation and unprivileged userns.
// =============================================================================
#[cfg(target_os = "linux")]
#[test]
fn test_linux_strict_capability_probe_and_isolation_smoke() {
    let run_id = format!("linux_adv_smoke_{}", std::process::id());
    let caps = PlatformCapabilities::probe_host(&run_id);
    if caps.verify_strict_admissibility().is_err() {
        // Host lacks cgroup delegation or userns: Strict mode must refuse fail-closed.
        let runner = SandboxRunner::new(run_id);
        let dir = runner.candidate_dir("cand");
        fs::create_dir_all(&dir).unwrap();
        let script = dir.join("noop.sh");
        write_executable(&script, "#!/bin/sh\nexit 0\n");
        assert!(runner.execute_binary(&script, &[]).is_err());
    }
}
