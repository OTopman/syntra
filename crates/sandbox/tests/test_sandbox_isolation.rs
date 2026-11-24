use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use syntra_sandbox::candidate::Candidate;
use syntra_sandbox::config::EnforcementMode;
use syntra_sandbox::runner::{RunOutput, SandboxRunner};
use syntra_sandbox::verifier::EquivalenceVerifier;

fn make_executable_script(path: &PathBuf, content: &str) {
    fs::write(path, content).expect("Failed to write script");
    let mut perms = fs::metadata(path)
        .expect("Failed to get perms")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("Failed to set executable mode");
}

#[test]
fn test_sandbox_execution_timeout_kills_infinite_loop() {
    let run_id = format!("test_timeout_{}", std::process::id());
    let runner = SandboxRunner::new(run_id)
        .with_enforcement_mode(EnforcementMode::Guarded)
        .with_timeout(Duration::from_millis(200));

    let candidate_dir = runner.candidate_dir("cand_infinite_loop");
    fs::create_dir_all(&candidate_dir).unwrap();
    let script_path = candidate_dir.join("infinite_loop.sh");

    // Adversarial infinite loop script
    make_executable_script(
        &script_path,
        "#!/bin/sh\nwhile true; do\n  sleep 0.1\ndone\n",
    );

    let t0 = Instant::now();
    let output = runner
        .execute_binary(&script_path, &[])
        .expect("Execution failed");
    let elapsed = t0.elapsed();

    // Verification: Child process must be killed within the timeout window
    assert!(output.timed_out, "Process should be flagged as timed out");
    assert!(
        elapsed < Duration::from_secs(2),
        "Timeout must abort loop in < 2s, took {elapsed:?}"
    );
    assert!(
        elapsed >= Duration::from_millis(180),
        "Timeout must allow process to run for configured duration, took {elapsed:?}"
    );
}

#[test]
fn test_sandbox_environment_isolation_prevents_secret_leakage() {
    // Inject a sensitive secret into parent host environment
    std::env::set_var("SYNTRA_SENSITIVE_API_KEY", "secret_token_abcdef123456");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "super_secret_aws_key");

    let run_id = format!("test_env_isolation_{}", std::process::id());
    let runner = SandboxRunner::new(run_id).with_enforcement_mode(EnforcementMode::Guarded);

    let candidate_dir = runner.candidate_dir("cand_inspect_env");
    fs::create_dir_all(&candidate_dir).unwrap();
    let script_path = candidate_dir.join("dump_env.sh");

    make_executable_script(
        &script_path,
        "#!/bin/sh\necho \"API_KEY=${SYNTRA_SENSITIVE_API_KEY}\"\necho \"AWS_KEY=${AWS_SECRET_ACCESS_KEY}\"\necho \"USER=${USER}\"\n",
    );

    let output = runner
        .execute_binary(&script_path, &[])
        .expect("Execution failed");

    // Verification: Host environment secrets must not leak to sandboxed candidate
    assert!(
        !output.stdout.contains("secret_token_abcdef123456"),
        "Sensitive API key must not leak to sandbox child"
    );
    assert!(
        !output.stdout.contains("super_secret_aws_key"),
        "AWS secret must not leak to sandbox child"
    );
    assert!(
        output.stdout.contains("API_KEY="),
        "Output must show empty cleared variable"
    );
    assert!(
        output.stdout.contains("USER=syntra_sandbox"),
        "User must be set to safe sandbox identity"
    );
}

#[test]
fn test_sandbox_output_truncation_limits_runaway_output() {
    let run_id = format!("test_trunc_{}", std::process::id());
    let runner = SandboxRunner::new(run_id)
        .with_enforcement_mode(EnforcementMode::Guarded)
        .with_max_output_bytes(256);

    let candidate_dir = runner.candidate_dir("cand_runaway_output");
    fs::create_dir_all(&candidate_dir).unwrap();
    let script_path = candidate_dir.join("runaway.sh");

    // Output generator emitting thousands of bytes
    make_executable_script(
        &script_path,
        "#!/bin/sh\nfor i in $(seq 1 500); do\n  echo \"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\"\ndone\n",
    );

    let output = runner
        .execute_binary(&script_path, &[])
        .expect("Execution failed");

    assert!(
        output.stdout.len() <= 256,
        "Output must be bounded to 256 bytes, was {} bytes",
        output.stdout.len()
    );
    assert!(
        !output.stdout.is_empty(),
        "Captured output should not be empty"
    );
}

#[test]
fn test_sandbox_directory_cleanup_on_drop() {
    let base_path: PathBuf;
    {
        let run_id = format!("test_drop_cleanup_{}", std::process::id());
        let runner = SandboxRunner::new(run_id);
        base_path = runner.sandbox_base().to_path_buf();

        let dir = runner.candidate_dir("cand_temp");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file.txt"), "temporary candidate data").unwrap();

        assert!(
            base_path.exists(),
            "Sandbox base directory should exist while runner is active"
        );
    } // runner is dropped here

    assert!(
        !base_path.exists(),
        "Sandbox directory must be completely removed when runner is dropped"
    );
}

#[test]
fn test_verifier_rejects_timed_out_candidate() {
    let baseline_output = RunOutput {
        stdout: "result: 42\n".to_string(),
        stderr: "".to_string(),
        exit_code: Some(0),
        duration: Duration::from_millis(10),
        timed_out: false,
    };

    let timed_out_candidate_output = RunOutput {
        stdout: "".to_string(),
        stderr: "".to_string(),
        exit_code: None,
        duration: Duration::from_millis(5000),
        timed_out: true,
    };

    let candidate = Candidate {
        id: "cand_hang".to_string(),
        name: "HangingCandidate".to_string(),
        strategy: syntra_sandbox::candidate::StrategyKind::Baseline,
        description: "Candidate that hangs in loop".to_string(),
        air_module: syntra_air::ir::AirModule {
            name: "test".to_string(),
            functions: Vec::new(),
            class_sizes: std::collections::HashMap::new(),
        },
    };

    let verif = EquivalenceVerifier::verify_differential_equivalence(
        &candidate,
        &baseline_output,
        &timed_out_candidate_output,
    );

    assert!(
        !verif.is_valid,
        "Timed-out candidate must be marked invalid"
    );
    assert!(
        verif
            .failure_reason
            .as_ref()
            .map(|r| r.contains("timed out"))
            .unwrap_or(false),
        "Failure reason must state candidate timed out"
    );
}
