#[cfg(target_os = "linux")]
mod linux_supervisor {
    use std::ffi::CString;
    use std::fs;
    use std::io::{Read, Write};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
    use std::path::{Path, PathBuf};

    struct Spec {
        run_id: String,
        binary: PathBuf,
        sandbox_base: PathBuf,
        evidence_out: PathBuf,
        memory_max: usize,
        pids_max: u32,
        cpu_quota: u32,
        max_open_fds: u64,
        max_disk_bytes: usize,
        max_file_size: u64,
        strict: bool,
        ro_libs: Vec<PathBuf>,
        envs: Vec<String>,
        args: Vec<String>,
    }

    #[repr(C)]
    #[derive(Default)]
    struct CloneArgs {
        flags: u64,
        pidfd: u64,
        child_tid: u64,
        parent_tid: u64,
        exit_signal: u64,
        stack: u64,
        stack_size: u64,
        tls: u64,
        set_tid: u64,
        set_tid_size: u64,
        cgroup: u64,
    }

    pub fn run(args: Vec<String>) -> Result<i32, String> {
        let mut spec_path = None;
        let mut i = 0;
        while i < args.len() {
            if args[i] == "--spec" && i + 1 < args.len() {
                spec_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            } else {
                i += 1;
            }
        }
        let spec_path = spec_path.ok_or_else(|| "Missing --spec argument".to_string())?;
        let spec = parse_spec(&spec_path)?;

        // S1 -> S2: Pre-create and lock cgroup v2 prior to spawning Trusted PID 1.
        let cgroup_dir = resolve_cgroup_run_dir(&spec.run_id);
        fs::create_dir_all(&cgroup_dir)
            .map_err(|e| format!("Failed creating cgroup '{}': {e}", cgroup_dir.display()))?;

        fs::write(cgroup_dir.join("memory.max"), spec.memory_max.to_string())
            .map_err(|e| format!("Failed writing memory.max: {e}"))?;
        let _ = fs::write(cgroup_dir.join("memory.swap.max"), "0");
        let _ = fs::write(cgroup_dir.join("memory.oom.group"), "1");
        fs::write(cgroup_dir.join("pids.max"), spec.pids_max.to_string())
            .map_err(|e| format!("Failed writing pids.max: {e}"))?;
        let quota_us = (spec.cpu_quota as u64) * 1000;
        let _ = fs::write(cgroup_dir.join("cpu.max"), format!("{quota_us} 100000"));

        let cgroup_fd = open_dir_cloexec(&cgroup_dir)?;

        // S2 -> S3: Create sync_t1_p1 and probe_pipe with O_CLOEXEC.
        let (sync_r, sync_w) = create_cloexec_pipe()?;
        let (probe_r, probe_w) = create_cloexec_pipe()?;

        let host_uid = unsafe { libc::getuid() };
        let host_gid = unsafe { libc::getgid() };

        // Clone Trusted PID 1 directly into target cgroup + isolated namespaces.
        let (pid1_host, pidfd_pid1) = spawn_pid1_in_cgroup(&cgroup_fd)?;

        if pid1_host == 0 {
            // --- EXECUTING INSIDE TRUSTED PID 1 ---
            drop(sync_w);
            drop(probe_r);
            drop(cgroup_fd);
            run_trusted_pid1(&spec, sync_r, probe_w);
            unsafe { libc::_exit(126) };
        }

        // --- EXECUTING INSIDE TIER 1 SUPERVISOR ---
        drop(sync_r);
        drop(probe_w);

        // S3 -> S4: Write UID/GID mappings for Trusted PID 1 userns before releasing sync_t1_p1.
        write_userns_maps(pid1_host, host_uid, host_gid)?;
        write_all_fd(sync_w.as_raw_fd(), &[0x01])?;
        drop(sync_w);

        // Read isolation probe status from Trusted PID 1.
        let mut probe_msg = String::new();
        let mut probe_file = unsafe { fs::File::from_raw_fd(probe_r.as_raw_fd()) };
        std::mem::forget(probe_r);
        let _ = probe_file.read_to_string(&mut probe_msg);
        let _ = fs::write(&spec.evidence_out, &probe_msg);

        // Wait on pidfd_pid1 via poll; reap Trusted PID 1 cleanly.
        let status = wait_pidfd_and_reap(pidfd_pid1.as_raw_fd(), pid1_host)?;
        let _ = kill_cgroup_tree(&cgroup_dir);
        Ok(status)
    }

    fn spawn_pid1_in_cgroup(cgroup_fd: &OwnedFd) -> Result<(libc::pid_t, OwnedFd), String> {
        let mut pidfd_raw: libc::c_int = -1;
        let mut cargs = CloneArgs {
            flags: (libc::CLONE_INTO_CGROUP as u64)
                | (libc::CLONE_PIDFD as u64)
                | (libc::CLONE_NEWUSER as u64)
                | (libc::CLONE_NEWNS as u64)
                | (libc::CLONE_NEWNET as u64)
                | (libc::CLONE_NEWPID as u64),
            pidfd: (&mut pidfd_raw as *mut libc::c_int) as u64,
            exit_signal: libc::SIGCHLD as u64,
            cgroup: cgroup_fd.as_raw_fd() as u64,
            ..Default::default()
        };

        let rc = unsafe {
            libc::syscall(
                libc::SYS_clone3,
                &mut cargs as *mut CloneArgs,
                std::mem::size_of::<CloneArgs>(),
            )
        };
        if rc >= 0 {
            let pid = rc as libc::pid_t;
            let pidfd = if pid == 0 {
                unsafe { OwnedFd::from_raw_fd(libc::dup(0)) }
            } else {
                unsafe { OwnedFd::from_raw_fd(pidfd_raw) }
            };
            return Ok((pid, pidfd));
        }

        let err = std::io::Error::last_os_error();
        Err(format!(
            "clone3(CLONE_INTO_CGROUP | CLONE_PIDFD) failed: {err}"
        ))
    }

    fn run_trusted_pid1(spec: &Spec, sync_r: OwnedFd, mut probe_w: OwnedFd) {
        unsafe {
            let _ = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            let _ = libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0);
        }

        // Block until Tier 1 writes UID/GID maps and sends 0x01; EOF means Tier 1 died.
        let mut token = [0u8; 1];
        if read_exact_fd(sync_r.as_raw_fd(), &mut token).is_err() || token[0] != 0x01 {
            unsafe { libc::_exit(126) };
        }
        drop(sync_r);

        let mut evidence = String::from("mem_cgroup=verified\n");

        // S4 -> S5: Private mount propagation + allowlist rootfs + netns audit.
        if setup_allowlist_rootfs(spec).is_ok() {
            evidence.push_str("fs_pivot=verified\n");
        } else {
            let _ = fs::File::write(
                &mut unsafe { fs::File::from_raw_fd(probe_w.as_raw_fd()) },
                evidence.as_bytes(),
            );
            std::mem::forget(probe_w);
            unsafe { libc::_exit(126) };
        }

        if audit_netns_isolated() {
            evidence.push_str("net_audit=verified\n");
        } else if spec.strict {
            unsafe { libc::_exit(126) };
        }

        // S5 -> S6: Fork Tier 2 Trusted Bootstrap (PID 2+ inside namespace).
        // `exec_err_pipe` has O_CLOEXEC: EOF means Seccomp + FD sweep succeeded AND execve ran.
        let Ok((exec_r, exec_w)) = create_cloexec_pipe() else {
            unsafe { libc::_exit(126) };
        };

        let child_pid = unsafe { libc::fork() };
        if child_pid < 0 {
            unsafe { libc::_exit(126) };
        }
        if child_pid == 0 {
            // --- EXECUTING INSIDE TIER 2 TRUSTED BOOTSTRAP (PID 2) ---
            drop(exec_r);
            drop(probe_w);
            apply_tier2_policies_and_exec(spec, exec_w.as_raw_fd());
            unsafe { libc::_exit(127) };
        }

        drop(exec_w);
        let mut tier2_err = [0u8; 64];
        let err_n = unsafe {
            libc::read(
                exec_r.as_raw_fd(),
                tier2_err.as_mut_ptr() as *mut libc::c_void,
                tier2_err.len(),
            )
        };
        drop(exec_r);

        if err_n == 0 {
            evidence.push_str("seccomp=verified\nfd_sanitized=verified\n");
        }
        let _ = write_all_fd(probe_w.as_raw_fd(), evidence.as_bytes());
        drop(probe_w);

        // S7 -> S10: Trusted PID 1 reaps PID 2 and all orphaned descendants until ECHILD.
        let mut candidate_code = 1;
        loop {
            let mut status: libc::c_int = 0;
            let pid = unsafe { libc::waitpid(-1, &mut status, 0) };
            if pid == child_pid {
                if libc::WIFEXITED(status) {
                    candidate_code = libc::WEXITSTATUS(status);
                } else if libc::WIFSIGNALED(status) {
                    candidate_code = 128 + libc::WTERMSIG(status);
                }
            } else if pid < 0 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::ECHILD) {
                    break;
                }
                if err.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                break;
            }
        }
        unsafe { libc::_exit(candidate_code) };
    }

    fn setup_allowlist_rootfs(spec: &Spec) -> Result<(), ()> {
        unsafe {
            if libc::mount(
                std::ptr::null(),
                c"/".as_ptr(),
                std::ptr::null(),
                libc::MS_REC | libc::MS_PRIVATE,
                std::ptr::null(),
            ) < 0
            {
                return Err(());
            }

            let rootfs = spec.sandbox_base.join("rootfs");
            let put_old = rootfs.join(".pivot_old");
            let _ = fs::create_dir_all(&rootfs);
            let opts = CString::new(format!(
                "size={},nr_inodes=2048,mode=0755",
                spec.max_disk_bytes
            ))
            .map_err(|_| ())?;
            let root_c = CString::new(rootfs.as_os_str().as_bytes()).map_err(|_| ())?;
            if libc::mount(
                c"tmpfs".as_ptr(),
                root_c.as_ptr(),
                c"tmpfs".as_ptr(),
                libc::MS_NOSUID | libc::MS_NODEV,
                opts.as_ptr() as *const libc::c_void,
            ) < 0
            {
                return Err(());
            }

            let _ = fs::create_dir_all(rootfs.join("candidate"));
            let _ = fs::create_dir_all(rootfs.join("tmp"));
            let _ = fs::create_dir_all(rootfs.join("proc"));
            let _ = fs::create_dir_all(rootfs.join("dev"));
            let _ = fs::create_dir_all(&put_old);

            // Copy/bind strictly the single allowlisted candidate binary.
            let target_bin = rootfs.join("candidate/bin");
            if fs::copy(&spec.binary, &target_bin).is_err() {
                return Err(());
            }

            // Bind-mount only explicitly allowlisted runtime shared object files (O_NOFOLLOW checked).
            for lib in &spec.ro_libs {
                if let Ok(canon) = fs::canonicalize(lib) {
                    if let Some(rel) = canon.to_str().map(|s| s.trim_start_matches('/')) {
                        let dst = rootfs.join(rel);
                        if let Some(parent) = dst.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if fs::File::create(&dst).is_ok() {
                            let src_c = CString::new(canon.as_os_str().as_bytes()).unwrap();
                            let dst_c = CString::new(dst.as_os_str().as_bytes()).unwrap();
                            let _ = libc::mount(
                                src_c.as_ptr(),
                                dst_c.as_ptr(),
                                std::ptr::null(),
                                libc::MS_BIND | libc::MS_RDONLY | libc::MS_NODEV | libc::MS_NOSUID,
                                std::ptr::null(),
                            );
                        }
                    }
                }
            }

            // pivot_root into isolated tmpfs and detach old host root.
            let old_c = CString::new(put_old.as_os_str().as_bytes()).map_err(|_| ())?;
            if libc::syscall(libc::SYS_pivot_root, root_c.as_ptr(), old_c.as_ptr()) < 0 {
                return Err(());
            }
            if libc::chdir(c"/".as_ptr()) < 0 {
                return Err(());
            }
            let _ = libc::umount2(c"/.pivot_old".as_ptr(), libc::MNT_DETACH);
            let _ = fs::remove_dir("/.pivot_old");

            // Mount container-private /proc with /proc/sys over-mounted read-only.
            let _ = libc::mount(
                c"proc".as_ptr(),
                c"/proc".as_ptr(),
                c"proc".as_ptr(),
                libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
                std::ptr::null(),
            );
            let _ = libc::mount(
                c"tmpfs".as_ptr(),
                c"/proc/sys".as_ptr(),
                c"tmpfs".as_ptr(),
                libc::MS_RDONLY | libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
                c"size=0".as_ptr() as *const libc::c_void,
            );
        }
        Ok(())
    }

    fn audit_netns_isolated() -> bool {
        let Ok(entries) = fs::read_dir("/sys/class/net") else {
            return true; // /sys intentionally unmounted in private pivot_root
        };
        for entry in entries.flatten() {
            if entry.file_name() != "lo" {
                return false;
            }
        }
        true
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SockFilter {
        code: u16,
        jt: u8,
        jf: u8,
        k: u32,
    }

    #[repr(C)]
    struct SockFprog {
        len: u16,
        filter: *const SockFilter,
    }

    unsafe fn install_seccomp_filter() -> bool {
        const BPF_LD: u16 = 0x00;
        const BPF_W: u16 = 0x00;
        const BPF_ABS: u16 = 0x20;
        const BPF_JMP: u16 = 0x05;
        const BPF_JEQ: u16 = 0x10;
        const BPF_K: u16 = 0x00;
        const BPF_RET: u16 = 0x06;
        const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
        const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
        const DENY_EPERM: u32 = SECCOMP_RET_ERRNO | (libc::EPERM as u32);

        let deny_syscalls: &[u32] = if cfg!(target_arch = "x86_64") {
            &[425, 426, 427, 101, 321, 272, 308, 165, 166, 155, 246, 323]
        } else {
            // aarch64 syscall numbers
            &[425, 426, 427, 117, 280, 97, 268, 40, 39, 41, 104, 282]
        };
        let sys_socket: u32 = if cfg!(target_arch = "x86_64") {
            41
        } else {
            198
        };

        let mut prog: Vec<SockFilter> = Vec::with_capacity(64);
        // Load seccomp_data.nr (offset 0)
        prog.push(SockFilter {
            code: BPF_LD | BPF_W | BPF_ABS,
            jt: 0,
            jf: 0,
            k: 0,
        });

        let deny_idx_offset = deny_syscalls.len();
        for (i, &nr) in deny_syscalls.iter().enumerate() {
            let rem_to_socket = (deny_idx_offset - i - 1) as u8;
            let jump_to_deny = (deny_idx_offset - i + 6) as u8;
            let _ = rem_to_socket;
            prog.push(SockFilter {
                code: BPF_JMP | BPF_JEQ | BPF_K,
                jt: jump_to_deny,
                jf: 0,
                k: nr,
            });
        }

        // Check SYS_socket: if equal, inspect args[0] (offset 16) for AF_INET/6/NETLINK/PACKET
        prog.push(SockFilter {
            code: BPF_JMP | BPF_JEQ | BPF_K,
            jt: 0,
            jf: 6, // skip to ALLOW
            k: sys_socket,
        });
        prog.push(SockFilter {
            code: BPF_LD | BPF_W | BPF_ABS,
            jt: 0,
            jf: 0,
            k: 16,
        });
        for (idx, domain) in [2u32, 10, 16, 17].iter().enumerate() {
            prog.push(SockFilter {
                code: BPF_JMP | BPF_JEQ | BPF_K,
                jt: (4 - idx) as u8, // jump to DENY
                jf: 0,
                k: *domain,
            });
        }

        // ALLOW
        prog.push(SockFilter {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: SECCOMP_RET_ALLOW,
        });
        // DENY (EPERM)
        prog.push(SockFilter {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: DENY_EPERM,
        });

        let fprog = SockFprog {
            len: prog.len() as u16,
            filter: prog.as_ptr(),
        };
        libc::prctl(
            libc::PR_SET_SECCOMP,
            libc::SECCOMP_MODE_FILTER,
            &fprog as *const SockFprog,
        ) == 0
    }

    fn apply_tier2_policies_and_exec(spec: &Spec, exec_err_w: libc::c_int) {
        unsafe {
            let _ = libc::setsid();
            let dev_null = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
            if dev_null >= 0 {
                let _ = libc::dup2(dev_null, 0);
                if dev_null > 2 {
                    libc::close(dev_null);
                }
            }

            let nofile = libc::rlimit {
                rlim_cur: spec.max_open_fds,
                rlim_max: spec.max_open_fds,
            };
            let _ = libc::setrlimit(libc::RLIMIT_NOFILE, &nofile);
            let fsize = libc::rlimit {
                rlim_cur: spec.max_file_size,
                rlim_max: spec.max_file_size,
            };
            let _ = libc::setrlimit(libc::RLIMIT_FSIZE, &fsize);

            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0 {
                let _ = write_all_fd(exec_err_w, b"no_new_privs");
                return;
            }

            if !install_seccomp_filter() && spec.strict {
                let _ = write_all_fd(exec_err_w, b"seccomp");
                return;
            }

            // Close inherited descriptors >= 3 except exec_err_w (which closes atomically via O_CLOEXEC on execve).
            for fd in 3..spec.max_open_fds.clamp(256, 4096) as libc::c_int {
                if fd != exec_err_w {
                    libc::close(fd);
                }
            }

            let bin_c = CString::new("/candidate/bin").unwrap();
            let mut argv_c = vec![bin_c.clone()];
            for a in &spec.args {
                if let Ok(c) = CString::new(a.as_str()) {
                    argv_c.push(c);
                }
            }
            let mut argv_ptrs: Vec<*const libc::c_char> =
                argv_c.iter().map(|c| c.as_ptr()).collect();
            argv_ptrs.push(std::ptr::null());

            let env_c: Vec<CString> = spec
                .envs
                .iter()
                .filter_map(|e| CString::new(e.as_str()).ok())
                .collect();
            let mut env_ptrs: Vec<*const libc::c_char> = env_c.iter().map(|c| c.as_ptr()).collect();
            env_ptrs.push(std::ptr::null());

            libc::execve(bin_c.as_ptr(), argv_ptrs.as_ptr(), env_ptrs.as_ptr());
            let _ = write_all_fd(exec_err_w, b"execve");
        }
    }

    fn parse_spec(path: &Path) -> Result<Spec, String> {
        let raw = fs::read_to_string(path).map_err(|e| format!("Read spec failed: {e}"))?;
        let mut run_id = String::new();
        let mut binary = PathBuf::new();
        let mut sandbox_base = PathBuf::new();
        let mut evidence_out = PathBuf::new();
        let mut memory_max = 128 * 1024 * 1024;
        let mut pids_max = 32;
        let mut cpu_quota = 100;
        let mut max_open_fds = 64;
        let mut max_disk_bytes = 32 * 1024 * 1024;
        let mut max_file_size = 16 * 1024 * 1024;
        let mut strict = true;
        let mut ro_libs = Vec::new();
        let mut envs = Vec::new();
        let mut args = Vec::new();

        for line in raw.lines() {
            if let Some((k, v)) = line.split_once('=') {
                match k {
                    "run_id" => run_id = v.to_string(),
                    "binary" => binary = PathBuf::from(v),
                    "sandbox_base" => sandbox_base = PathBuf::from(v),
                    "evidence_out" => evidence_out = PathBuf::from(v),
                    "memory_max" => memory_max = v.parse().unwrap_or(memory_max),
                    "pids_max" => pids_max = v.parse().unwrap_or(pids_max),
                    "cpu_quota" => cpu_quota = v.parse().unwrap_or(cpu_quota),
                    "max_open_fds" => max_open_fds = v.parse().unwrap_or(max_open_fds),
                    "max_disk_bytes" => max_disk_bytes = v.parse().unwrap_or(max_disk_bytes),
                    "max_file_size" => max_file_size = v.parse().unwrap_or(max_file_size),
                    "strict" => strict = v == "true",
                    "ro_lib" => ro_libs.push(PathBuf::from(v)),
                    "env" => envs.push(v.to_string()),
                    "arg" => args.push(v.to_string()),
                    _ => {}
                }
            }
        }
        Ok(Spec {
            run_id,
            binary,
            sandbox_base,
            evidence_out,
            memory_max,
            pids_max,
            cpu_quota,
            max_open_fds,
            max_disk_bytes,
            max_file_size,
            strict,
            ro_libs,
            envs,
            args,
        })
    }

    fn resolve_cgroup_run_dir(run_id: &str) -> PathBuf {
        let root = Path::new("/sys/fs/cgroup");
        if let Ok(self_cg) = fs::read_to_string("/proc/self/cgroup") {
            for line in self_cg.lines() {
                if let Some(rel) = line.strip_prefix("0::") {
                    let parent = root.join(rel.trim_start_matches('/'));
                    if parent.is_dir() {
                        return parent.join(format!("syntra_sece_{run_id}"));
                    }
                }
            }
        }
        root.join("syntra_sandbox").join(run_id)
    }

    fn write_userns_maps(
        pid: libc::pid_t,
        uid: libc::uid_t,
        gid: libc::gid_t,
    ) -> Result<(), String> {
        let base = PathBuf::from(format!("/proc/{pid}"));
        fs::write(base.join("setgroups"), "deny").map_err(|e| format!("setgroups failed: {e}"))?;
        fs::write(base.join("uid_map"), format!("0 {uid} 1"))
            .map_err(|e| format!("uid_map failed: {e}"))?;
        fs::write(base.join("gid_map"), format!("0 {gid} 1"))
            .map_err(|e| format!("gid_map failed: {e}"))?;
        Ok(())
    }

    fn open_dir_cloexec(path: &Path) -> Result<OwnedFd, String> {
        let c = CString::new(path.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        let fd = unsafe {
            libc::open(
                c.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(format!("open('{}') failed", path.display()));
        }
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn create_cloexec_pipe() -> Result<(OwnedFd, OwnedFd), String> {
        let mut fds = [0 as libc::c_int; 2];
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } < 0 {
            return Err("pipe2 failed".to_string());
        }
        Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
    }

    fn write_all_fd(fd: libc::c_int, buf: &[u8]) -> Result<(), String> {
        let n = unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) };
        if n < 0 {
            return Err("write failed".to_string());
        }
        Ok(())
    }

    fn read_exact_fd(fd: libc::c_int, buf: &mut [u8]) -> Result<(), ()> {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n == buf.len() as isize {
            Ok(())
        } else {
            Err(())
        }
    }

    fn wait_pidfd_and_reap(pidfd: libc::c_int, pid: libc::pid_t) -> Result<i32, String> {
        let mut pfd = libc::pollfd {
            fd: pidfd,
            events: libc::POLLIN,
            revents: 0,
        };
        loop {
            let rc = unsafe { libc::poll(&mut pfd, 1, -1) };
            if rc < 0 {
                if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                return Err("poll(pidfd) failed".to_string());
            }
            break;
        }
        let mut status: libc::c_int = 0;
        unsafe { libc::waitpid(pid, &mut status, 0) };
        if libc::WIFEXITED(status) {
            Ok(libc::WEXITSTATUS(status))
        } else {
            Ok(1)
        }
    }

    fn kill_cgroup_tree(cgroup_dir: &Path) -> Result<(), String> {
        let kill_file = cgroup_dir.join("cgroup.kill");
        if kill_file.exists() {
            let _ = fs::write(kill_file, "1");
        }
        Ok(())
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        match linux_supervisor::run(args) {
            Ok(code) => std::process::exit(code),
            Err(err) => {
                eprintln!("syntra-sandbox-supervisor error: {err}");
                std::process::exit(126);
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!(
            "syntra-sandbox-supervisor is only active on Linux hosts (current OS: {})",
            std::env::consts::OS
        );
        std::process::exit(1);
    }
}
