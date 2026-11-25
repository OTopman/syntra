# Syntra Phase 2: SECE Sandbox Security Architecture
# Deliverable 06C: Implementation Sequencing & Migration Plan

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6 — Implementation Sequencing & Migration Plan  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **STAGE 6 PLAN — READY FOR REVIEW**

---

## 1. Executive Summary & Purpose

Refactoring security-critical runtime code requires a structured, dependency-ordered execution plan. Introducing isolation primitives in an ad-hoc manner risks breaking the 118 existing workspace tests, introducing silent security regressions, or bricking backward compatibility with `syntra_cli`.

This document establishes the **staged migration plan** for executing Stage 7 (Controlled Implementation) and Stage 8 (Adversarial Verification) once formal architectural authorization is granted.

---

## 2. Dependency-Ordered Implementation Sequence

The migration is decomposed into six self-contained, sequentially validated milestones:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        STAGE 7 IMPLEMENTATION PHASES                   │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  [ Milestone 7.1: Core Types & Error Hierarchy ]                       │
│    • error.rs, config.rs, traits.rs                                    │
│    • Validated via unit tests; zero changes to existing execution      │
│                     │                                                  │
│                     ▼                                                  │
│  [ Milestone 7.2: Dynamic Capability Probing Engine ]                  │
│    • capabilities.rs, platform/mod.rs, platform/fallback.rs            │
│    • Runtime kernel probing; fail-closed fallback                      │
│                     │                                                  │
│                     ▼                                                  │
│  [ Milestone 7.3: Event-Driven Stream Supervisor ]                     │
│    • supervisor.rs (non-blocking poll, bounded buffers)                │
│    • Independent stdout/stderr drain loop, deadline enforcement        │
│                     │                                                  │
│                     ▼                                                  │
│  [ Milestone 7.4: Native OS Platform Providers ]                       │
│    • 7.4A: platform/macos.rs   (Process groups, Guarded mode)          │
│    • 7.4B: platform/linux.rs   (Namespaces, Cgroups v2, Seccomp)       │
│    • 7.4C: platform/windows.rs (Job Objects, Memory Limits)            │
│                     │                                                  │
│                     ▼                                                  │
│  [ Milestone 7.5: Runner Adapter & Engine Wiring ]                     │
│    • Refactor runner.rs to delegate to PlatformProvider                │
│    • 100% backward compatible public API                               │
│                     │                                                  │
│                     ▼                                                  │
│  [ Milestone 7.6: Regression Baseline Verification ]                   │
│    • All 118 existing workspace tests pass cleanly                     │
│    • Clippy zero warnings, clean formatting                            │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Detailed Stage 7 Milestone Specifications

### Milestone 7.1: Foundational Abstractions & Error Hierarchy
- **Targets:** `error.rs`, `config.rs`, `traits.rs`
- **Actions:**
  - Define `SandboxError` with detailed display formatting and error source mapping.
  - Define `EnforcementMode` (`Strict`, `Guarded`), `SandboxConfig`, and `StreamBudgetConfig`.
  - Define `SandboxProvider` and `IsolatedChild` traits.
- **Verification Gate:**
  - Unit tests for configuration builders, defaults, and error formatting.
  - Zero modifications to existing `runner.rs` or `lib.rs`.

### Milestone 7.2: Capability Engine & Fallback Provider
- **Targets:** `capabilities.rs`, `platform/mod.rs`, `platform/fallback.rs`
- **Actions:**
  - Implement 4-state `CapabilityState` enum and `PlatformCapabilities` struct.
  - Implement runtime dynamic detection:
    - Linux: probe user namespace sysctl, cgroups v2 controller availability.
    - macOS: probe process-group primitives; mark filesystem/network `Unavailable`.
    - Windows: probe Job Object support.
  - Implement `FallbackProvider`: unconditionally refuses `Strict` execution with `StrictInvariantViolation`.
- **Verification Gate:**
  - Capability unit tests verifying that macOS/Windows correctly report `Unavailable` for mount/network isolation.

### Milestone 7.3: Non-Blocking Stream Supervisor
- **Targets:** `supervisor.rs`
- **Actions:**
  - Implement non-blocking polling loop using `poll()` with monotonic time slices.
  - Implement bounded `StreamBuffer` collecting up to `limit_bytes`.
  - Implement `overflow_detected` flag triggering immediate child termination.
  - Implement 500 ms drain window on child termination.
- **Verification Gate:**
  - Mock stream tests verifying no thread deadlocks when pipe write end is held open by descendant.

### Milestone 7.4: Platform Provider Implementation
- **Milestone 7.4A (macOS Provider - `platform/macos.rs`):**
  - Implement `setpgid(0, 0)` in `pre_exec`.
  - Implement `kill(-pgid, SIGKILL)` in `kill_tree()`.
  - Rejection of `EnforcementMode::Strict` with `StrictInvariantViolation`.
  - Guarded mode audit warning output.
- **Milestone 7.4B (Linux Provider - `platform/linux.rs`):**
  - Implement user namespace UID/GID mapping (0 -> current UID).
  - Implement private mount namespace pivot to minimal tmpfs.
  - Implement network namespace unshare with loopback interface verification.
  - Implement Cgroups v2 `memory.max`, `memory.swap.max = 0`, `memory.oom.group = 1`, and `pids.max`.
  - Implement Seccomp-BPF filter blocking `io_uring` and unprivileged socket creation.
  - Implement `sync_pipe` handshake and `pidfd` tracking.
- **Milestone 7.4C (Windows Provider - `platform/windows.rs`):**
  - Implement Job Object creation with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and `JOB_OBJECT_LIMIT_JOB_MEMORY`.
  - Suspended process creation and job association.
  - Rejection of `EnforcementMode::Strict` due to missing filesystem/network virtualization.

### Milestone 7.5: Runner Adapter & Facade Integration
- **Targets:** `runner.rs`, `lib.rs`
- **Actions:**
  - Refactor `SandboxRunner::execute_binary(&self, ...)` to:
    1. Resolve active `SandboxProvider` via `platform::get_default_provider()`.
    2. Convert `SandboxRunner` options into `SandboxConfig`.
    3. Spawn candidate via `provider.spawn(...)`.
    4. Supervise process via `ProcessSupervisor::supervise(...)`.
    5. Convert execution results into existing `RunOutput`.
  - Preserve `cleanup()` and `Drop` implementation.
- **Verification Gate:**
  - Existing 5 unit tests in `test_sandbox_isolation.rs` pass without code changes.

### Milestone 7.6: Full Workspace Regression
- **Targets:** Entire Syntra compiler workspace
- **Actions:**
  - Run `cargo test --workspace`.
  - Confirm all 118 existing tests pass cleanly.
  - Run `cargo clippy --all-targets -- -D warnings`.
  - Run `cargo fmt -- --check`.

---

## 4. Existing Code Migration Strategy

To avoid disrupting existing consumers, the refactored `SandboxRunner` will act as a facade over the new provider architecture:

```rust
// runner.rs Migration Strategy:
pub struct SandboxRunner {
    sandbox_base: PathBuf,
    run_id: String,
    timeout: Duration,
    max_output_bytes: usize,
    enforcement_mode: EnforcementMode, // Defaults to Guarded on macOS/Windows, Strict on Linux
    env_vars: Vec<(String, String)>,
}

impl SandboxRunner {
    pub fn execute_binary(&self, binary_path: &Path, args: &[String]) -> Result<RunOutput, String> {
        let mut config = SandboxConfig::new(self.run_id.clone());
        config.wall_clock_timeout = self.timeout;
        config.streams.stdout_limit_bytes = self.max_output_bytes;
        config.streams.stderr_limit_bytes = self.max_output_bytes;
        config.enforcement_mode = self.enforcement_mode;
        config.env_vars = self.env_vars.clone();

        let provider = platform::get_default_provider();
        let mut child = provider.spawn(binary_path, args, &config)
            .map_err(|e| e.to_string())?;

        let run_result = ProcessSupervisor::supervise(&mut *child, &config.streams, config.wall_clock_timeout);
        
        // Map to RunOutput format
        Ok(RunOutput {
            stdout: run_result.stdout,
            stderr: run_result.stderr,
            exit_code: run_result.exit_code,
            duration: run_result.duration,
            timed_out: run_result.timed_out,
        })
    }
}
```

---

## 5. Security-Critical Implementation Boundaries

1. **Default Enforcement Mode:**
   - In production SECE candidate generation, `enforcement_mode` is **Strict by default**.
   - On native macOS and Windows, `syn experiment` refuses to run untrusted candidate code unless the user explicitly passes `--guarded`.
2. **Third-Party Crate Boundaries:**
   - Linux and macOS backends require low-level syscalls.
   - `libc` dependency will be added under `[target.'cfg(unix)'.dependencies]` in `crates/sandbox/Cargo.toml`.
   - Windows Job Objects will be added under `[target.'cfg(windows)'.dependencies]` using `windows-sys`.
   - No unnecessary third-party async runtime (e.g. Tokio) will be introduced; the supervisor uses pure, lightweight synchronous POSIX `poll()` / eventfd polling.

---

## 6. Stage 8 Adversarial Test Dependencies

Stage 8 will implement the 10 adversarial scenarios defined in Deliverables 04 and 05:

| Adversarial Scenario | Required Platform Implementation | Expected Milestone Validation |
| :--- | :--- | :---: |
| **ADV-01** (Double-fork daemon) | `CLONE_NEWPID` / Job Object / `setpgid` | Milestone 7.4 |
| **ADV-02** (Credential access) | `CLONE_NEWNS` tmpfs pivot | Milestone 7.4B |
| **ADV-03** (Network egress) | `CLONE_NEWNET` unshare | Milestone 7.4B |
| **ADV-04** (Memory bomb) | Cgroups v2 `memory.max` + `memory.swap.max = 0` | Milestone 7.4B / 7.4C |
| **ADV-05** (Fork bomb) | Cgroups v2 `pids.max` | Milestone 7.4B / 7.4C |
| **ADV-06** (FD hijacking) | `close_range` sanitization | Milestone 7.4 |
| **ADV-07** (`io_uring` evasion) | Seccomp-BPF filter | Milestone 7.4B |
| **ADV-08** (Stream flood) | Non-blocking `supervisor.rs` | Milestone 7.3 |
| **ADV-09** (Fail-closed strict) | Dynamic capability conjunction | Milestone 7.2 |
| **ADV-10** (Procfs masking) | Read-only `/proc/sys` mount | Milestone 7.4B |

---

## 7. Rollback & Regression Defense Strategy

- **Atomic Commits:** Each milestone (7.1 through 7.6) will be committed separately only after passing the complete 118-test regression suite.
- **Rollback Safety:** If any platform backend introduces compilation issues or regressions on a target OS, the `FallbackProvider` can be designated default, immediately restoring compiler functionality while preserving fail-closed security.
- **Evidence Trail:** Full `cargo test` logs will be saved into `remediation_output/evidence/` for every milestone.

*Deliverable 06C complete. Proceeding to Deliverable 06D.*
