use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_syn_binary() -> PathBuf {
    let mut bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    bin.pop(); // crates/
    bin.pop(); // aether/
    bin.push("target");
    bin.push("debug");
    bin.push("syn");
    bin
}

#[test]
fn test_approval_gate_manifest_and_build_integration() {
    let syn = get_syn_binary();
    assert!(syn.is_file(), "syn binary must exist at {}", syn.display());

    let tmp = std::env::temp_dir().join(format!("syntra_test_approval_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let syn_src = tmp.join("test_app.syn");
    fs::write(
        &syn_src,
        r#"
fn main(): int {
    int a = 10;
    int b = 32;
    return a + b;
}
"#,
    )
    .unwrap();

    // 1. Approve candidate 1 (EscapePromoted)
    let approve_output = Command::new(&syn)
        .arg("approve")
        .arg("cand_1_escape_wpa")
        .arg("--dir")
        .arg(&tmp)
        .output()
        .expect("Failed to execute syn approve");

    assert!(approve_output.status.success());
    let stdout = String::from_utf8_lossy(&approve_output.stdout);
    assert!(stdout.contains("Successfully approved strategy 'cand_1_escape_wpa'"));

    let approval_file = tmp.join(".syntra").join("approved_strategy.json");
    assert!(
        approval_file.is_file(),
        "approved_strategy.json must be created"
    );

    // 2. Build binary and verify that syn build consumes the approved strategy
    let bin_out = tmp.join("test_app_bin");
    let build_output = Command::new(&syn)
        .arg("build")
        .arg(&syn_src)
        .arg("-o")
        .arg(&bin_out)
        .current_dir(&tmp)
        .output()
        .expect("Failed to execute syn build");

    assert!(build_output.status.success());
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);
    assert!(
        build_stdout.contains("Approved Strategy: Consuming '.syntra/approved_strategy.json'"),
        "syn build must consume approved_strategy.json: {build_stdout}"
    );
    assert!(
        build_stdout.contains("Candidate: 'cand_1_escape_wpa'"),
        "syn build output must display candidate id: {build_stdout}"
    );

    // 3. Verify the generated binary runs successfully
    let run_output = Command::new(&bin_out)
        .output()
        .expect("Failed to run binary");
    assert_eq!(run_output.status.code(), Some(42));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_approval_baseline_candidate_disables_optimization() {
    let syn = get_syn_binary();
    assert!(syn.is_file());

    let tmp = std::env::temp_dir().join(format!(
        "syntra_test_approval_baseline_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let syn_src = tmp.join("test_baseline.syn");
    fs::write(
        &syn_src,
        r#"
fn main(): int {
    return 100;
}
"#,
    )
    .unwrap();

    // Approve baseline candidate (0 passes)
    let approve_output = Command::new(&syn)
        .arg("approve")
        .arg("cand_0_baseline")
        .arg("--dir")
        .arg(&tmp)
        .output()
        .expect("Failed to execute syn approve");
    assert!(approve_output.status.success());

    let bin_out = tmp.join("test_baseline_bin");
    let build_output = Command::new(&syn)
        .arg("build")
        .arg(&syn_src)
        .arg("-o")
        .arg(&bin_out)
        .current_dir(&tmp)
        .output()
        .expect("Failed to execute syn build");

    assert!(build_output.status.success());
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);
    assert!(build_stdout.contains("Approved Strategy: Consuming '.syntra/approved_strategy.json'"));
    assert!(build_stdout.contains("Optimization pipeline applied 0 pass(es)"));

    let _ = fs::remove_dir_all(&tmp);
}
