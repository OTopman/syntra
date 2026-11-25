# Syntra Phase 2: SECE Sandbox Security Assessment
# Deliverable 01: Evidence-Based Assessment of Existing `crates/sandbox` Implementation

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox  
**Crate Path:** `crates/sandbox`  
**Audited Commit Hash:** [`c9f5c1e103a73b24a9c7d133cec2ba1b6af56fcc`](https://github.com/OTopman/syntra/commit/c9f5c1e103a73b24a9c7d133cec2ba1b6af56fcc) (tag: `phase1-accepted`) on branch `develop`  
**Auditor:** Principal Compiler Security Engineer & Systems Runtime Auditor  
**Date:** 2026-11-24  
**Audit Finding:** **INSUFFICIENT ISOLATION — PROCESS-LEVEL WRAPPER ONLY (FIND-002 REOPENED)**

---

## 1. Executive Summary & Objective

In accordance with the Phase 2 milestone directive, this document provides an **empirical, line-by-line evidence-based security assessment of the existing `crates/sandbox` implementation** before any redesign or architectural changes are proposed.

The remediation register originally marked **FIND-002** ("SECE Pseudo-Sandbox") as `Resolved (Verified)` based on adding execution timeouts, environment variable clearing, and bounded stream reading. However, an independent technical audit of the codebase confirms that **the current implementation is not a secure sandbox**. It executes candidate machine code directly on the host as an ordinary child process under the developer's user identity (UID), with:
- **Zero filesystem confinement** (full host read/write access under developer UID).
- **Zero network boundaries** (full outbound socket and local network probing access).
- **Zero memory quotas** (vulnerable to host OOM denial of service).
- **Zero process tree containment** (forked children and detached daemons evade termination).
- **Zero syscall filtering** (no Seccomp on Linux, no Seatbelt on macOS).

Passing the existing 5 unit tests proves that the test harness functions as written; it does not establish that untrusted code cannot compromise the host.

---

## 2. Architectural Role of `crates/sandbox` in Syntra

In the Syntra architecture, `crates/sandbox` implements the **Autonomous Experimental Compilation Engine (SECE / Subsystem 07)**.

Its functional workflow is:
1. **Candidate Generation (`candidate.rs`):** Takes an AIR module emitted by the frontend and generates 4 speculative optimization variants (`Baseline`, `EscapePromoted`, `ConstantFolded`, `AggressiveCombined`).
2. **Compilation (`runner.rs:compile_candidate`):** Calls `syntra_backend_cranelift` to emit machine code object files (`candidate.o`) and invokes the host system linker to produce native executables (`candidate_bin`).
3. **Execution & Equivalence Verification (`verifier.rs`):** Executes candidate executables via `SandboxRunner::execute_binary`, comparing stdout and exit codes against the unoptimized baseline to detect semantic divergence.
4. **Pareto Benchmarking (`benchmark.rs`):** Executes verified candidates across multiple iterations, records microsecond latencies, compile duration, and binary size, and computes the multi-objective Pareto optimal frontier.
5. **Approval (`lib.rs:approve_candidate`):** Commits the winning optimization strategy into `.syntra/approved_strategy.json`, which `syn build` subsequently loads during production builds.

Because SECE compiles and runs untrusted, autonomously generated or speculative code variations, the runner is the primary execution boundary guarding the host machine.

---

## 3. Code-Level Inspection of `crates/sandbox/src/runner.rs`

The entire "sandboxing" mechanism is concentrated in `SandboxRunner` in `crates/sandbox/src/runner.rs`.

### 3.1 Process Spawning Model (`runner.rs:134-162`)

```rust
let mut cmd = Command::new(binary_path);
cmd.args(args);
cmd.env_clear();

// Minimal required system paths
let system_path =
    std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin:/usr/local/bin".to_string());
cmd.env("PATH", system_path);
cmd.env("TMPDIR", parent_dir);
cmd.env("HOME", parent_dir);
cmd.env("USER", "syntra_sandbox");
cmd.env("LC_ALL", "C");

// Explicit environment variables (if configured)
for (k, v) in &self.env_vars {
    cmd.env(k, v);
}

cmd.stdout(Stdio::piped());
cmd.stderr(Stdio::piped());

let mut child = cmd.spawn()?;
```

#### Empirical Evidence & Analysis:
1. **Direct Host Process Execution:** `Command::new(binary_path).spawn()` issues standard OS `fork()`/`execve()` directly in the host user's security context. It does not invoke any container runtime, namespace unsharing, or kernel confinement.
2. **The `HOME` / `TMPDIR` Environment Fallacy:**
   - Setting `cmd.env("HOME", parent_dir)` and `cmd.env("TMPDIR", parent_dir)` alters userland environment variables only.
   - Standard libc system calls (`open("/etc/passwd", O_RDONLY)`, `open("/Users/user/.ssh/id_rsa", O_RDONLY)`) and direct syscalls do not inspect the `HOME` environment variable. Any candidate process can read or write any path on the host filesystem permitted by the developer's UID.
3. **Inherited `PATH`:** Setting `PATH` to the host's `/usr/bin:/bin:/usr/local/bin` allows candidate binaries to execute host binaries (e.g., `curl`, `nc`, `rm`, `sh`, `python`).

---

### 3.2 Process Termination & Timeout Mechanism (`runner.rs:188-212`)

```rust
// Polling loop with timeout check
let mut timed_out = false;
let mut exit_code = None;

loop {
    match child.try_wait() {
        Ok(Some(status)) => {
            exit_code = status.code();
            break;
        }
        Ok(None) => {
            if t0.elapsed() >= self.timeout {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait(); // reap zombie
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Error polling candidate process: {e}"));
        }
    }
}
```

#### Empirical Evidence & Analysis:
1. **Single-PID Kill Limitation:**
   `child.kill()` sends `SIGKILL` **exclusively to the direct child PID**.
2. **Fork-Bomb & Daemon Detachment Escape:**
   If a candidate program executes a double-fork or spawns background child processes (`fork()`, `clone()`, `posix_spawn()`, or `system()`):
   - The direct child exits or is killed.
   - The grandchildren are re-parented to `init` (PID 1 on Linux) or `launchd` on macOS.
   - `child.kill()` fails to terminate the detached processes, leaving malicious or runaway worker processes executing indefinitely on the host.
3. **No Process Group Tracking:**
   `cmd.spawn()` does not create a new process group (`setpgid(0, 0)`), nor does `child.kill()` send a kill signal to the process group (`kill(-pgid, SIGKILL)`).
4. **Wall-Clock vs CPU Time:**
   The timeout is measured via `Instant::now()` wall-clock time. If the host system experiences heavy load, legitimate candidates can falsely time out, while rogue multi-threaded programs can consume 100% of all host CPU cores during the 5-second window.

---

### 3.3 Stream Bounding (`runner.rs:164-185`)

```rust
let stdout_thread = thread::spawn(move || {
    let mut buf = Vec::new();
    if let Some(mut stream) = stdout_handle.take() {
        let mut limited = (&mut stream).take(max_output as u64);
        let _ = limited.read_to_end(&mut buf);
    }
    buf
});
```

#### Empirical Evidence & Analysis:
- `(&mut stream).take(max_output as u64)` prevents buffer exhaustion inside the parent runner process when candidates emit infinite output.
- **Limitation:** This only limits pipe reading. It does not prevent the candidate process from writing gigabytes of data directly to files in `/tmp` or the current directory, which can exhaust host disk space.

---

## 4. Evaluation of Security Boundaries (The 6 Sandbox Deficits)

The table below contrasts the current implementation against standard secure compiler sandbox requirements:

| Security Boundary | Secure Sandbox Requirement | Existing `crates/sandbox` Implementation | Status |
| :--- | :--- | :--- | :--- |
| **1. Filesystem Isolation** | Ephemeral private root, read-only system mounts, denial of access to `~/.ssh`, `~/.aws`, source code | **None.** Full host filesystem read/write access under host UID. Only `TMPDIR` and `HOME` env vars are redirected. | **CRITICAL GAP (0% Confinement)** |
| **2. Network Boundary** | Denial of raw sockets, no outbound internet access, no localhost loopback probing | **None.** Shares host network stack. Can open outbound sockets, exfiltrate files, or probe internal network ports. | **CRITICAL GAP (0% Confinement)** |
| **3. Memory Limits** | Kernel-enforced memory ceiling (RSS) preventing host OOM panic | **None.** No `RLIMIT_AS` or cgroup memory limits. Candidate can allocate all host RAM. | **CRITICAL GAP (OOM Vulnerability)** |
| **4. CPU / Thread Limits** | Execution-time limits on CPU core count and thread spawning | **None.** No `RLIMIT_CPU` or cgroup CPU limits. Candidates can spawn unlimited threads and saturate all host cores. | **HIGH RISK (DoS Vulnerability)** |
| **5. Process Containment** | Process group / PID namespace confinement preventing fork-bombs and daemon escapes | **None.** Only direct child PID is tracked. Descendant processes survive `child.kill()`. No `RLIMIT_NPROC`. | **CRITICAL GAP (Daemon Escape)** |
| **6. Syscall Filtering** | Restricted syscall surface (Seccomp-BPF on Linux, Seatbelt on macOS) | **None.** All host OS syscalls (`ptrace`, `socket`, `fork`, `execve`) permitted without restriction. | **CRITICAL GAP (Full OS Surface)** |

---

## 5. Empirical Analysis of Existing Tests (`tests/test_sandbox_isolation.rs`)

The existing integration test suite contains 5 tests:

1. **`test_sandbox_execution_timeout_kills_infinite_loop`:**
   - *Test Code:* Spawns a single-process shell script with `while true; do sleep 0.1; done`.
   - *Result:* Passes because a single process without children responds to `child.kill()`.
   - *Deficit:* Fails to test forked child processes, daemon detachment, or fork bombs.
2. **`test_sandbox_environment_isolation_prevents_secret_leakage`:**
   - *Test Code:* Sets `SYNTRA_SENSITIVE_API_KEY` in parent environment, runs a script that prints `$SYNTRA_SENSITIVE_API_KEY`, and checks that stdout does not contain the key.
   - *Result:* Passes because `cmd.env_clear()` cleared the environment table.
   - *Deficit:* Fails to test reading secret files from disk (`~/.ssh/id_rsa`, `~/.aws/credentials`), which are completely readable.
3. **`test_sandbox_output_truncation_limits_runaway_output`:**
   - *Test Code:* Runs a script printing 500 lines of text with a 256-byte limit.
   - *Result:* Passes because `.take(256)` limits parent stream consumption.
   - *Deficit:* Fails to test direct disk writes or runaway memory allocation.
4. **`test_sandbox_directory_cleanup_on_drop`:**
   - *Test Code:* Asserts `runner.cleanup()` deletes the directory when dropped.
   - *Result:* Passes via RAII `Drop`.
5. **`test_verifier_rejects_timed_out_candidate`:**
   - *Test Code:* Asserts `EquivalenceVerifier` flags `timed_out: true` as invalid.
   - *Result:* Passes logic check.

### Conclusion on Existing Tests:
The test suite validates wrapper ergonomics (env clearing, pipe truncation, directory cleanup). It does **not** test hostile code containment or sandbox resistance.

---

## 6. Structural Vulnerabilities Register (Phase 2 Baseline)

The empirical audit establishes the following specific findings in the current implementation:

1. **FIND-SECE-01 (Severity: CRITICAL): Unconfined Host Filesystem Access.**
   A candidate executing native code or scripts can read, modify, or delete developer source code, SSH keys, credentials, and OS configuration.
2. **FIND-SECE-02 (Severity: CRITICAL): Unrestricted Host Network Access.**
   A candidate can initiate outbound TCP/UDP connections to remote servers or probe local services (e.g. `localhost:8080`, database ports).
3. **FIND-SECE-03 (Severity: CRITICAL): Incomplete Process Tree Termination.**
   `child.kill()` does not terminate descendant processes, allowing detached daemons and background tasks to persist on the host.
4. **FIND-SECE-04 (Severity: HIGH): Lack of Memory & Resource Quotas.**
   Unconstrained heap allocation can exhaust host physical RAM and swap, causing host system instability.
5. **FIND-SECE-05 (Severity: CRITICAL): Unrestricted Syscall Surface.**
   Without Seccomp-BPF (Linux) or Seatbelt (macOS), candidate code has unrestricted access to the host kernel interface.

---

## 7. Next Step in Phase 2 Protocol

This evidence-based assessment confirms ChatGPT's warning: **marking FIND-002 as resolved was premature, and the existing implementation is merely a process-level wrapper.**

Before any implementation changes are made, the next deliverable must be:
- **Phase 2 Deliverable 02: SECE Sandbox Threat Model & Multi-OS Architecture Specification.**
- Defining explicit platform support (Linux user namespaces/Landlock/cgroups vs macOS Seatbelt/sandbox-exec vs Windows Job Objects), formal capability boundaries, and adversarial escape tests.
