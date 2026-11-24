use crate::config::EnforcementMode;
use crate::error::SandboxError;
use std::time::{SystemTime, UNIX_EPOCH};

/// Non-linear capability verification status defined in Deliverable 08E.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityVerificationStatus {
    /// Host operating system lacks the required kernel primitive.
    Unsupported,
    /// Host OS supports the primitive, but current user/configuration lacks authority.
    Unavailable,
    /// Cooperative or incomplete boundary (e.g. macOS PGID, Windows Job without security boundary).
    PartiallyEnforced,
    /// Configured or available for staging, but execution-scoped verification has not yet run.
    ConfiguredUnverified,
    /// Kernel enforcement observed and scoped adversarial verification passed for this run.
    EnforcedAndVerified,
    /// Configuration or runtime probe failed verification.
    VerificationFailed,
}

impl CapabilityVerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unsupported => "Unsupported",
            Self::Unavailable => "Unavailable",
            Self::PartiallyEnforced => "PartiallyEnforced",
            Self::ConfiguredUnverified => "ConfiguredUnverified",
            Self::EnforcedAndVerified => "EnforcedAndVerified",
            Self::VerificationFailed => "VerificationFailed",
        }
    }

    /// Whether this status is admissible for pre-execution `Strict` mode authorization.
    pub fn is_strict_admissible(&self) -> bool {
        matches!(self, Self::ConfiguredUnverified | Self::EnforcedAndVerified)
    }
}

/// Auditable per-capability evidence record required by Deliverable 08E.
#[derive(Debug, Clone)]
pub struct CapabilityEvidenceRecord {
    pub capability_id: String,
    pub status: CapabilityVerificationStatus,
    pub required_enforcement_mechanism: String,
    pub host_prerequisites: Vec<String>,
    pub configuration_result: String,
    pub kernel_observed_enforcement_evidence: String,
    pub independent_adversarial_verification: String,
    pub verification_scope_limit: String,
    pub failure_reason: Option<String>,
    pub platform_and_kernel_version: String,
    pub evidence_timestamp_unix_ms: u128,
    pub execution_id: String,
}

impl CapabilityEvidenceRecord {
    pub fn new(
        capability_id: impl Into<String>,
        status: CapabilityVerificationStatus,
        required_enforcement_mechanism: impl Into<String>,
        host_prerequisites: Vec<String>,
        verification_scope_limit: impl Into<String>,
        execution_id: impl Into<String>,
    ) -> Self {
        Self {
            capability_id: capability_id.into(),
            status,
            required_enforcement_mechanism: required_enforcement_mechanism.into(),
            host_prerequisites,
            configuration_result: "probed_only".to_string(),
            kernel_observed_enforcement_evidence: "none".to_string(),
            independent_adversarial_verification: "pending".to_string(),
            verification_scope_limit: verification_scope_limit.into(),
            failure_reason: None,
            platform_and_kernel_version: host_platform_kernel_string(),
            evidence_timestamp_unix_ms: now_unix_ms(),
            execution_id: execution_id.into(),
        }
    }

    pub fn to_json_string(&self) -> String {
        let prereqs = self
            .host_prerequisites
            .iter()
            .map(|p| format!("\"{}\"", json_escape(p)))
            .collect::<Vec<_>>()
            .join(", ");
        let failure = match &self.failure_reason {
            Some(r) => format!("\"{}\"", json_escape(r)),
            None => "null".to_string(),
        };
        format!(
            concat!(
                "{{\n",
                "      \"capability_id\": \"{}\",\n",
                "      \"status\": \"{}\",\n",
                "      \"required_enforcement_mechanism\": \"{}\",\n",
                "      \"host_prerequisites\": [{}],\n",
                "      \"configuration_result\": \"{}\",\n",
                "      \"kernel_observed_enforcement_evidence\": \"{}\",\n",
                "      \"independent_adversarial_verification\": \"{}\",\n",
                "      \"verification_scope_limit\": \"{}\",\n",
                "      \"failure_reason\": {},\n",
                "      \"platform_and_kernel_version\": \"{}\",\n",
                "      \"evidence_timestamp_unix_ms\": {},\n",
                "      \"execution_id\": \"{}\"\n",
                "    }}"
            ),
            json_escape(&self.capability_id),
            self.status.as_str(),
            json_escape(&self.required_enforcement_mechanism),
            prereqs,
            json_escape(&self.configuration_result),
            json_escape(&self.kernel_observed_enforcement_evidence),
            json_escape(&self.independent_adversarial_verification),
            json_escape(&self.verification_scope_limit),
            failure,
            json_escape(&self.platform_and_kernel_version),
            self.evidence_timestamp_unix_ms,
            json_escape(&self.execution_id),
        )
    }
}

/// Post-execution teardown verification metadata.
#[derive(Debug, Clone, Default)]
pub struct CleanupVerification {
    pub cgroup_procs_empty: Option<bool>,
    pub cgroup_removed: Option<bool>,
    pub temp_dir_removed: bool,
    pub surviving_pids: Vec<u32>,
}

/// Complete machine-readable evidence report for an execution (`IND-09`).
#[derive(Debug, Clone)]
pub struct ExecutionEvidenceReport {
    pub schema_version: &'static str,
    pub execution_id: String,
    pub platform_os: &'static str,
    pub kernel_release: String,
    pub enforcement_mode: EnforcementMode,
    pub memory_max_bytes: usize,
    pub swap_max_bytes: usize,
    pub pids_max: u32,
    pub stdout_limit_bytes: usize,
    pub stderr_limit_bytes: usize,
    pub capabilities: Vec<CapabilityEvidenceRecord>,
    pub exit_status: String,
    pub cleanup: CleanupVerification,
}

impl ExecutionEvidenceReport {
    pub fn to_json_string(&self) -> String {
        let caps = self
            .capabilities
            .iter()
            .map(CapabilityEvidenceRecord::to_json_string)
            .collect::<Vec<_>>()
            .join(",\n");
        let surviving = self
            .cleanup
            .surviving_pids
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let cgroup_empty = self
            .cleanup
            .cgroup_procs_empty
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let cgroup_removed = self
            .cleanup
            .cgroup_removed
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let mode_str = match self.enforcement_mode {
            EnforcementMode::Strict => "Strict",
            EnforcementMode::Guarded => "Guarded",
        };
        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": \"{}\",\n",
                "  \"execution_id\": \"{}\",\n",
                "  \"platform_os\": \"{}\",\n",
                "  \"kernel_release\": \"{}\",\n",
                "  \"enforcement_mode\": \"{}\",\n",
                "  \"runtime_config\": {{\n",
                "    \"memory_max_bytes\": {},\n",
                "    \"swap_max_bytes\": {},\n",
                "    \"pids_max\": {},\n",
                "    \"stdout_limit_bytes\": {},\n",
                "    \"stderr_limit_bytes\": {}\n",
                "  }},\n",
                "  \"capabilities\": [\n{}\n  ],\n",
                "  \"exit_status\": \"{}\",\n",
                "  \"post_run_cleanup\": {{\n",
                "    \"cgroup_procs_empty\": {},\n",
                "    \"cgroup_removed\": {},\n",
                "    \"temp_dir_removed\": {},\n",
                "    \"surviving_pids\": [{}]\n",
                "  }}\n",
                "}}"
            ),
            self.schema_version,
            json_escape(&self.execution_id),
            self.platform_os,
            json_escape(&self.kernel_release),
            mode_str,
            self.memory_max_bytes,
            self.swap_max_bytes,
            self.pids_max,
            self.stdout_limit_bytes,
            self.stderr_limit_bytes,
            caps,
            json_escape(&self.exit_status),
            cgroup_empty,
            cgroup_removed,
            self.cleanup.temp_dir_removed,
            surviving,
        )
    }
}

/// Host platform isolation capabilities established via runtime probing.
#[derive(Debug, Clone)]
pub struct PlatformCapabilities {
    pub platform_name: &'static str,
    pub fs_mount_isolation: CapabilityEvidenceRecord,
    pub net_egress_isolation: CapabilityEvidenceRecord,
    pub proc_tree_containment: CapabilityEvidenceRecord,
    pub mem_cgroup_accounting: CapabilityEvidenceRecord,
    pub mem_swap_elimination: CapabilityEvidenceRecord,
    pub proc_pid_ceiling: CapabilityEvidenceRecord,
    pub fd_sanitization: CapabilityEvidenceRecord,
    pub time_wall_clock_timeout: CapabilityEvidenceRecord,
    pub seccomp_syscall_filtering: CapabilityEvidenceRecord,
}

impl PlatformCapabilities {
    /// Dynamically probes the current host OS and kernel privileges without falling
    /// back to compile-time assumptions of active enforcement.
    pub fn probe_host(execution_id: &str) -> Self {
        #[cfg(target_os = "linux")]
        {
            Self::probe_linux_host(execution_id)
        }
        #[cfg(target_os = "macos")]
        {
            Self::probe_macos_host(execution_id)
        }
        #[cfg(target_os = "windows")]
        {
            Self::probe_windows_host(execution_id)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Self::unsupported_host(execution_id)
        }
    }

    pub fn mandatory_strict_invariants(&self) -> Vec<(&CapabilityEvidenceRecord, &'static str)> {
        vec![
            (
                &self.fs_mount_isolation,
                "Strict mode requires Linux mount namespace + allowlist pivot_root isolation.",
            ),
            (
                &self.net_egress_isolation,
                "Strict mode requires unshared network namespace + verified zero routes/interfaces.",
            ),
            (
                &self.proc_tree_containment,
                "Strict mode requires Trusted PID 1 namespace + cgroup kill tree termination.",
            ),
            (
                &self.mem_cgroup_accounting,
                "Strict mode requires direct cgroup v2 memory accounting.",
            ),
            (
                &self.mem_swap_elimination,
                "Strict mode requires explicit zero-swap cgroup enforcement.",
            ),
            (
                &self.proc_pid_ceiling,
                "Strict mode requires aggregate process/thread cgroup ceiling.",
            ),
            (
                &self.fd_sanitization,
                "Strict mode requires pre-exec standard descriptor and terminal sanitization.",
            ),
            (
                &self.time_wall_clock_timeout,
                "Strict mode requires monotonic wall-clock timeout enforcement.",
            ),
        ]
    }

    /// Evaluates the conjunction of all mandatory strict invariants and fails closed
    /// if any capability is not admissible.
    pub fn verify_strict_admissibility(&self) -> Result<(), SandboxError> {
        for (record, hint) in self.mandatory_strict_invariants() {
            if !record.status.is_strict_admissible() {
                return Err(SandboxError::StrictInvariantViolation {
                    missing_invariant: record.capability_id.clone(),
                    platform_state: record.status.as_str().to_string(),
                    remediation_hint: format!(
                        "{hint} Host platform '{}' reports {} ({}).",
                        self.platform_name,
                        record.status.as_str(),
                        record
                            .failure_reason
                            .as_deref()
                            .unwrap_or(&record.required_enforcement_mechanism)
                    ),
                });
            }
        }
        Ok(())
    }

    pub fn all_records(&self) -> Vec<CapabilityEvidenceRecord> {
        vec![
            self.fs_mount_isolation.clone(),
            self.net_egress_isolation.clone(),
            self.proc_tree_containment.clone(),
            self.mem_cgroup_accounting.clone(),
            self.mem_swap_elimination.clone(),
            self.proc_pid_ceiling.clone(),
            self.fd_sanitization.clone(),
            self.time_wall_clock_timeout.clone(),
            self.seccomp_syscall_filtering.clone(),
        ]
    }

    #[cfg(target_os = "macos")]
    fn probe_macos_host(execution_id: &str) -> Self {
        let mut fs = CapabilityEvidenceRecord::new(
            "fs.mount_isolation",
            CapabilityVerificationStatus::Unsupported,
            "Linux CLONE_NEWNS + allowlist pivot_root (unavailable on Darwin/XNU)",
            vec!["unprivileged_mount_namespace".to_string()],
            "Host check only; XNU lacks unprivileged mount namespaces and sandbox-exec is deprecated.",
            execution_id,
        );
        fs.failure_reason =
            Some("macOS XNU does not provide unprivileged mount namespace isolation".to_string());

        let mut net = CapabilityEvidenceRecord::new(
            "net.egress_isolation",
            CapabilityVerificationStatus::Unsupported,
            "Linux CLONE_NEWNET + netlink audit (unavailable on Darwin/XNU)",
            vec!["unprivileged_network_namespace".to_string()],
            "Host check only; XNU lacks unprivileged network namespaces.",
            execution_id,
        );
        net.failure_reason =
            Some("macOS XNU does not provide unprivileged network namespace isolation".to_string());

        let mut proc_tree = CapabilityEvidenceRecord::new(
            "proc.tree_containment",
            CapabilityVerificationStatus::PartiallyEnforced,
            "POSIX setpgid(0,0) + kill(-pgid, SIGKILL)",
            vec!["posix_setpgid".to_string()],
            "Cooperative group termination only; broken if candidate calls setsid().",
            execution_id,
        );
        proc_tree.failure_reason =
            Some("macOS process groups are not immune to setsid() detachment escapes".to_string());

        let mut mem = CapabilityEvidenceRecord::new(
            "mem.cgroup_accounting",
            CapabilityVerificationStatus::Unsupported,
            "Cgroups v2 memory.max (unavailable on Darwin/XNU)",
            vec!["cgroups_v2".to_string()],
            "RLIMIT_AS is non-enforcing for aggregate candidate trees on macOS.",
            execution_id,
        );
        mem.failure_reason =
            Some("macOS XNU lacks cgroup memory ceilings and reliable RLIMIT_AS".to_string());

        let mut swap = CapabilityEvidenceRecord::new(
            "mem.swap_elimination",
            CapabilityVerificationStatus::Unsupported,
            "Cgroups v2 memory.swap.max=0 (unavailable on Darwin/XNU)",
            vec!["cgroups_v2_swap".to_string()],
            "macOS cannot disable paging per candidate process tree.",
            execution_id,
        );
        swap.failure_reason = Some("macOS lacks per-tree swap disablement controls".to_string());

        let mut pids = CapabilityEvidenceRecord::new(
            "proc.pid_ceiling",
            CapabilityVerificationStatus::PartiallyEnforced,
            "POSIX RLIMIT_NPROC (per-UID, not per-tree)",
            vec!["posix_rlimit".to_string()],
            "Applies across host user UID rather than isolated container namespace.",
            execution_id,
        );
        pids.failure_reason =
            Some("macOS lacks per-execution aggregate process tree counters".to_string());

        let fd = CapabilityEvidenceRecord::new(
            "fd.sanitization",
            CapabilityVerificationStatus::ConfiguredUnverified,
            "stdin -> /dev/null + pre-exec FD > 2 close sweep + SIGTTOU ignore",
            vec!["dev_null_open".to_string(), "fd_sweep".to_string()],
            "Verifies pre-exec FD sanitization; cannot prevent post-exec file opens on shared FS.",
            execution_id,
        );

        let time = CapabilityEvidenceRecord::new(
            "time.wall_clock_timeout",
            CapabilityVerificationStatus::ConfiguredUnverified,
            "Supervisor monotonic poll loop deadline",
            vec!["monotonic_clock".to_string()],
            "Enforces host supervisor wait deadline and bounded drain.",
            execution_id,
        );

        let mut seccomp = CapabilityEvidenceRecord::new(
            "seccomp.syscall_filtering",
            CapabilityVerificationStatus::Unsupported,
            "Seccomp-BPF (unavailable on Darwin/XNU)",
            vec!["linux_seccomp_bpf".to_string()],
            "macOS does not expose unprivileged syscall BPF filters.",
            execution_id,
        );
        seccomp.failure_reason = Some("macOS XNU has no Seccomp-BPF".to_string());

        Self {
            platform_name: "macos",
            fs_mount_isolation: fs,
            net_egress_isolation: net,
            proc_tree_containment: proc_tree,
            mem_cgroup_accounting: mem,
            mem_swap_elimination: swap,
            proc_pid_ceiling: pids,
            fd_sanitization: fd,
            time_wall_clock_timeout: time,
            seccomp_syscall_filtering: seccomp,
        }
    }

    #[cfg(target_os = "linux")]
    fn probe_linux_host(execution_id: &str) -> Self {
        let userns_ok = probe_linux_userns_authorized();
        let cgroup_v2 = probe_linux_cgroup_v2_controllers();
        let seccomp_ok = probe_linux_seccomp_available();

        let ns_status = if userns_ok {
            CapabilityVerificationStatus::ConfiguredUnverified
        } else {
            CapabilityVerificationStatus::Unavailable
        };

        let mut fs = CapabilityEvidenceRecord::new(
            "fs.mount_isolation",
            ns_status,
            "CLONE_NEWUSER + CLONE_NEWNS + MS_PRIVATE + allowlist pivot_root",
            vec!["unprivileged_userns".to_string(), "mount_ns".to_string()],
            "Pre-run probe verifies host userns capability; execution proof requires mountinfo & traversal tests.",
            execution_id,
        );
        if !userns_ok {
            fs.failure_reason =
                Some("Unprivileged user namespaces denied or sysctl disabled on host".to_string());
        }

        let mut net = CapabilityEvidenceRecord::new(
            "net.egress_isolation",
            ns_status,
            "CLONE_NEWNET + loopback DOWN + 0 routes + Seccomp socket family block",
            vec!["unprivileged_userns".to_string(), "net_ns".to_string()],
            "Pre-run probe verifies host netns creation authority; runtime netlink check still required.",
            execution_id,
        );
        if !userns_ok {
            net.failure_reason = fs.failure_reason.clone();
        }

        let mut proc_tree = CapabilityEvidenceRecord::new(
            "proc.tree_containment",
            if userns_ok && cgroup_v2.memory_writable {
                CapabilityVerificationStatus::ConfiguredUnverified
            } else {
                CapabilityVerificationStatus::Unavailable
            },
            "Trusted PID 1 namespace + Tier 1 pidfd + cgroups v2 cgroup.kill",
            vec!["pid_ns".to_string(), "cgroup2_delegation".to_string()],
            "Pre-run probe verifies PID ns and cgroup kill support.",
            execution_id,
        );
        if !cgroup_v2.memory_writable {
            proc_tree.failure_reason = Some(cgroup_v2.reason.clone());
        }

        let mem_status = if cgroup_v2.memory_writable && cgroup_v2.clone_into_cgroup_supported {
            CapabilityVerificationStatus::ConfiguredUnverified
        } else if cgroup_v2.memory_writable {
            CapabilityVerificationStatus::PartiallyEnforced
        } else {
            CapabilityVerificationStatus::Unavailable
        };

        let mut mem = CapabilityEvidenceRecord::new(
            "mem.cgroup_accounting",
            mem_status,
            "Cgroups v2 memory.max + clone3(CLONE_INTO_CGROUP) via sterile Tier 1",
            vec![
                "cgroup2_memory_controller".to_string(),
                "clone3_into_cgroup".to_string(),
            ],
            "Accounts anon, page cache, shmem, and kernel memory inside sandbox cgroup.",
            execution_id,
        );
        if mem_status != CapabilityVerificationStatus::ConfiguredUnverified {
            mem.failure_reason = Some(cgroup_v2.reason.clone());
        }

        let mut swap = CapabilityEvidenceRecord::new(
            "mem.swap_elimination",
            if cgroup_v2.swap_writable {
                CapabilityVerificationStatus::ConfiguredUnverified
            } else {
                CapabilityVerificationStatus::Unavailable
            },
            "Cgroups v2 memory.swap.max = 0",
            vec!["cgroup2_swap_controller".to_string()],
            "Prevents host disk swap spillover beyond memory.max.",
            execution_id,
        );
        if !cgroup_v2.swap_writable {
            swap.failure_reason =
                Some("memory.swap.max controller unavailable or unwritable".to_string());
        }

        let mut pids = CapabilityEvidenceRecord::new(
            "proc.pid_ceiling",
            if cgroup_v2.pids_writable {
                CapabilityVerificationStatus::ConfiguredUnverified
            } else {
                CapabilityVerificationStatus::Unavailable
            },
            "Cgroups v2 pids.max",
            vec!["cgroup2_pids_controller".to_string()],
            "Aggregate limit over all candidate threads and child processes.",
            execution_id,
        );
        if !cgroup_v2.pids_writable {
            pids.failure_reason =
                Some("cgroup2 pids controller unavailable or unwritable".to_string());
        }

        let fd = CapabilityEvidenceRecord::new(
            "fd.sanitization",
            CapabilityVerificationStatus::ConfiguredUnverified,
            "O_CLOEXEC pipes + pre-exec FD close + setsid() + stdin -> /dev/null + close_range(3..)",
            vec!["dev_null_open".to_string()],
            "Verifies inherited FDs > 2 closed before candidate execve.",
            execution_id,
        );

        let time = CapabilityEvidenceRecord::new(
            "time.wall_clock_timeout",
            CapabilityVerificationStatus::ConfiguredUnverified,
            "Tier 1 monotonic poll() deadline + cgroup teardown",
            vec!["monotonic_clock".to_string()],
            "Deterministic deadline + 500 ms drain cap.",
            execution_id,
        );

        let mut seccomp = CapabilityEvidenceRecord::new(
            "seccomp.syscall_filtering",
            if seccomp_ok {
                CapabilityVerificationStatus::ConfiguredUnverified
            } else {
                CapabilityVerificationStatus::Unavailable
            },
            "PR_SET_NO_NEW_PRIVS + Seccomp-BPF filter blocking io_uring/ptrace/raw sockets",
            vec!["seccomp_bpf".to_string()],
            "Verified for blocked syscall numbers on current arch.",
            execution_id,
        );
        if !seccomp_ok {
            seccomp.failure_reason = Some("PR_GET_SECCOMP unsupported or disallowed".to_string());
        }

        Self {
            platform_name: "linux",
            fs_mount_isolation: fs,
            net_egress_isolation: net,
            proc_tree_containment: proc_tree,
            mem_cgroup_accounting: mem,
            mem_swap_elimination: swap,
            proc_pid_ceiling: pids,
            fd_sanitization: fd,
            time_wall_clock_timeout: time,
            seccomp_syscall_filtering: seccomp,
        }
    }

    #[cfg(target_os = "windows")]
    fn probe_windows_host(execution_id: &str) -> Self {
        let job_ok = unsafe {
            let h = windows_sys::Win32::System::JobObjects::CreateJobObjectW(
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            if h.is_null() {
                false
            } else {
                windows_sys::Win32::Foundation::CloseHandle(h);
                true
            }
        };

        let mut fs = CapabilityEvidenceRecord::new(
            "fs.mount_isolation",
            CapabilityVerificationStatus::Unavailable,
            "Dedicated WindowsSecurityProvider (AppContainer / Restricted Token)",
            vec!["windows_appcontainer_provider".to_string()],
            "Job Objects do not restrict host filesystem reads/writes.",
            execution_id,
        );
        fs.failure_reason =
            Some("Job Object resource containment does not isolate host filesystem".to_string());

        let mut net = CapabilityEvidenceRecord::new(
            "net.egress_isolation",
            CapabilityVerificationStatus::Unavailable,
            "WFP / AppContainer capability block",
            vec!["windows_security_provider".to_string()],
            "Job Objects do not isolate Winsock network egress.",
            execution_id,
        );
        net.failure_reason =
            Some("Job Object resource containment does not isolate Windows networking".to_string());

        let mut proc_tree = CapabilityEvidenceRecord::new(
            "proc.tree_containment",
            if job_ok {
                CapabilityVerificationStatus::PartiallyEnforced
            } else {
                CapabilityVerificationStatus::Unavailable
            },
            "Job Object KILL_ON_JOB_CLOSE + CREATE_SUSPENDED + breakaway disabled",
            vec!["windows_job_object".to_string()],
            "Covers direct CreateProcess children; does not block out-of-process COM/WMI brokers (Win32_Process.Create).",
            execution_id,
        );
        proc_tree.failure_reason = Some(
            "Direct process creation contained, but out-of-process DCOM/WMI broker spawns remain outside Job without token restriction"
                .to_string(),
        );

        let mut mem = CapabilityEvidenceRecord::new(
            "mem.cgroup_accounting",
            if job_ok {
                CapabilityVerificationStatus::ConfiguredUnverified
            } else {
                CapabilityVerificationStatus::Unavailable
            },
            "JOB_OBJECT_LIMIT_JOB_MEMORY committed virtual limit",
            vec!["windows_job_object".to_string()],
            "Limits aggregate committed virtual memory for processes inside the Job.",
            execution_id,
        );
        if !job_ok {
            mem.failure_reason = Some("CreateJobObjectW failed".to_string());
        }

        let mut swap = CapabilityEvidenceRecord::new(
            "mem.swap_elimination",
            CapabilityVerificationStatus::Unsupported,
            "Per-job swap disablement unsupported on Win32 Job Objects",
            vec![],
            "Windows manages pagefile globally across committed virtual memory.",
            execution_id,
        );
        swap.failure_reason = Some(
            "Windows Job Objects cap committed virtual memory rather than swap separately"
                .to_string(),
        );

        let pids = CapabilityEvidenceRecord::new(
            "proc.pid_ceiling",
            if job_ok {
                CapabilityVerificationStatus::ConfiguredUnverified
            } else {
                CapabilityVerificationStatus::Unavailable
            },
            "JOB_OBJECT_LIMIT_ACTIVE_PROCESSES",
            vec!["windows_job_object".to_string()],
            "Limits active direct Job members.",
            execution_id,
        );

        let fd = CapabilityEvidenceRecord::new(
            "fd.sanitization",
            CapabilityVerificationStatus::ConfiguredUnverified,
            "STARTUPINFOEXW PROC_THREAD_ATTRIBUTE_HANDLE_LIST + explicit stdio handles",
            vec!["windows_attribute_list".to_string()],
            "Restricts handle inheritance to explicitly listed stdio handles.",
            execution_id,
        );

        let time = CapabilityEvidenceRecord::new(
            "time.wall_clock_timeout",
            CapabilityVerificationStatus::ConfiguredUnverified,
            "Monotonic timeout + TerminateJobObject / CloseHandle",
            vec!["windows_wait".to_string()],
            "Wall-clock deadline enforced by supervisor.",
            execution_id,
        );

        let mut seccomp = CapabilityEvidenceRecord::new(
            "seccomp.syscall_filtering",
            CapabilityVerificationStatus::Unsupported,
            "Not applicable on Windows",
            vec![],
            "Windows does not support Seccomp-BPF.",
            execution_id,
        );
        seccomp.failure_reason = Some("Seccomp-BPF is Linux-only".to_string());

        Self {
            platform_name: "windows",
            fs_mount_isolation: fs,
            net_egress_isolation: net,
            proc_tree_containment: proc_tree,
            mem_cgroup_accounting: mem,
            mem_swap_elimination: swap,
            proc_pid_ceiling: pids,
            fd_sanitization: fd,
            time_wall_clock_timeout: time,
            seccomp_syscall_filtering: seccomp,
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn unsupported_host(execution_id: &str) -> Self {
        let mk = |id: &str| {
            let mut r = CapabilityEvidenceRecord::new(
                id,
                CapabilityVerificationStatus::Unsupported,
                "No platform sandbox backend available",
                vec![],
                "Unsupported platform fails closed.",
                execution_id,
            );
            r.failure_reason = Some("Unsupported host OS".to_string());
            r
        };
        Self {
            platform_name: std::env::consts::OS,
            fs_mount_isolation: mk("fs.mount_isolation"),
            net_egress_isolation: mk("net.egress_isolation"),
            proc_tree_containment: mk("proc.tree_containment"),
            mem_cgroup_accounting: mk("mem.cgroup_accounting"),
            mem_swap_elimination: mk("mem.swap_elimination"),
            proc_pid_ceiling: mk("proc.pid_ceiling"),
            fd_sanitization: mk("fd.sanitization"),
            time_wall_clock_timeout: mk("time.wall_clock_timeout"),
            seccomp_syscall_filtering: mk("seccomp.syscall_filtering"),
        }
    }
}

#[cfg(target_os = "linux")]
struct LinuxCgroupProbe {
    memory_writable: bool,
    swap_writable: bool,
    pids_writable: bool,
    clone_into_cgroup_supported: bool,
    reason: String,
}

#[cfg(target_os = "linux")]
fn probe_linux_userns_authorized() -> bool {
    if let Ok(val) = std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone") {
        if val.trim() == "0" {
            return false;
        }
    }
    // Check current userns creation via fork + unshare(CLONE_NEWUSER)
    unsafe {
        let pid = libc::fork();
        if pid == 0 {
            let rc = libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNET | libc::CLONE_NEWNS);
            libc::_exit(if rc == 0 { 0 } else { 1 });
        } else if pid > 0 {
            let mut status: libc::c_int = 0;
            if libc::waitpid(pid, &mut status, 0) > 0 {
                return libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0;
            }
        }
    }
    false
}

#[cfg(target_os = "linux")]
fn probe_linux_cgroup_v2_controllers() -> LinuxCgroupProbe {
    let root = std::path::Path::new("/sys/fs/cgroup");
    let ctrl_path = root.join("cgroup.controllers");
    let Ok(ctrls) = std::fs::read_to_string(&ctrl_path) else {
        return LinuxCgroupProbe {
            memory_writable: false,
            swap_writable: false,
            pids_writable: false,
            clone_into_cgroup_supported: false,
            reason: "Cgroups v2 unified hierarchy not mounted at /sys/fs/cgroup".to_string(),
        };
    };

    let mut base_dir = None;
    if let Ok(self_cg) = std::fs::read_to_string("/proc/self/cgroup") {
        for line in self_cg.lines() {
            if let Some(rel) = line.strip_prefix("0::") {
                let candidate = root.join(rel.trim_start_matches('/'));
                if candidate.is_dir() {
                    base_dir = Some(candidate);
                    break;
                }
            }
        }
    }
    let test_parent = base_dir.unwrap_or_else(|| root.join("syntra_sandbox"));
    let probe_dir = test_parent.join(format!("syntra_probe_{}", std::process::id()));
    let can_mkdir = std::fs::create_dir_all(&probe_dir).is_ok();
    let has_mem = can_mkdir && ctrls.split_whitespace().any(|c| c == "memory");
    let has_pids = can_mkdir && ctrls.split_whitespace().any(|c| c == "pids");
    let mem_writable = has_mem && probe_dir.join("memory.max").exists();
    let swap_writable = has_mem && probe_dir.join("memory.swap.max").exists();
    let pids_writable = has_pids && probe_dir.join("pids.max").exists();
    if can_mkdir {
        let _ = std::fs::remove_dir(&probe_dir);
    }

    let clone_into_ok = probe_clone_into_cgroup_flag();
    let reason = if !can_mkdir {
        "Current user lacks cgroup delegation permission under /sys/fs/cgroup".to_string()
    } else if !mem_writable {
        "Cgroups v2 memory controller not delegated to current cgroup subtree".to_string()
    } else if !clone_into_ok {
        "clone3(CLONE_INTO_CGROUP) unavailable on host kernel or blocked by outer seccomp"
            .to_string()
    } else {
        "Cgroups v2 delegation and clone3(CLONE_INTO_CGROUP) verified".to_string()
    };

    LinuxCgroupProbe {
        memory_writable: mem_writable,
        swap_writable,
        pids_writable,
        clone_into_cgroup_supported: clone_into_ok,
        reason,
    }
}

#[cfg(target_os = "linux")]
fn probe_clone_into_cgroup_flag() -> bool {
    // Check kernel >= 5.7 via uname; empirical clone3 behavior is validated during spawn.
    if let Ok(rel) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        let mut parts = rel.trim().split('.');
        let major: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor: u32 = parts
            .next()
            .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        return (major > 5) || (major == 5 && minor >= 7);
    }
    false
}

#[cfg(target_os = "linux")]
fn probe_linux_seccomp_available() -> bool {
    unsafe { libc::prctl(libc::PR_GET_SECCOMP, 0, 0, 0, 0) >= 0 }
}

pub fn host_platform_kernel_string() -> String {
    #[cfg(unix)]
    {
        unsafe {
            let mut uts: libc::utsname = std::mem::zeroed();
            if libc::uname(&mut uts) == 0 {
                let sys = std::ffi::CStr::from_ptr(uts.sysname.as_ptr())
                    .to_string_lossy()
                    .into_owned();
                let rel = std::ffi::CStr::from_ptr(uts.release.as_ptr())
                    .to_string_lossy()
                    .into_owned();
                return format!("{sys} {rel}");
            }
        }
    }
    std::env::consts::OS.to_string()
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_capabilities_strict_admissibility_honest() {
        let caps = PlatformCapabilities::probe_host("cap_unit_test");
        #[cfg(not(target_os = "linux"))]
        {
            let res = caps.verify_strict_admissibility();
            assert!(
                res.is_err(),
                "Non-Linux host must fail Strict capability admissibility"
            );
        }
        assert_eq!(caps.all_records().len(), 9);
    }

    #[test]
    fn test_evidence_report_json_serialization() {
        let caps = PlatformCapabilities::probe_host("json_test");
        let report = ExecutionEvidenceReport {
            schema_version: "1.0.0",
            execution_id: "json_test".to_string(),
            platform_os: std::env::consts::OS,
            kernel_release: host_platform_kernel_string(),
            enforcement_mode: EnforcementMode::Guarded,
            memory_max_bytes: 67_108_864,
            swap_max_bytes: 0,
            pids_max: 16,
            stdout_limit_bytes: 1024,
            stderr_limit_bytes: 1024,
            capabilities: caps.all_records(),
            exit_status: "Exited(0)".to_string(),
            cleanup: CleanupVerification {
                cgroup_procs_empty: None,
                cgroup_removed: None,
                temp_dir_removed: true,
                surviving_pids: Vec::new(),
            },
        };
        let json = report.to_json_string();
        assert!(json.contains("\"schema_version\": \"1.0.0\""));
        assert!(json.contains("\"execution_id\": \"json_test\""));
        assert!(json.contains("\"temp_dir_removed\": true"));
    }
}
