use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn syn_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_syn"))
}

#[test]
#[allow(clippy::zombie_processes)]
fn test_self_hosting_loop_variable_codegen_and_execution() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let syn_self_src = repo_root.join("src/syn_self.syn");
    assert!(syn_self_src.exists(), "src/syn_self.syn must exist");

    let temp_dir = std::env::temp_dir().join(format!("syntra_self_host_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let syn_self_bin = temp_dir.join("syn_self_test");
    let cf_src = repo_root.join("examples/control_flow_test.syn");
    let cf_bin = temp_dir.join("cf_via_self");
    let cf_asm = temp_dir.join("cf_via_self.s");

    // 1. Compile src/syn_self.syn using the Rust bootstrap compiler (syn build)
    let build_status = Command::new(syn_bin())
        .arg("build")
        .arg(&syn_self_src)
        .arg("-o")
        .arg(&syn_self_bin)
        .current_dir(&repo_root)
        .status()
        .expect("Failed to invoke bootstrap compiler on syn_self.syn");
    assert!(build_status.success(), "Failed to build syn_self compiler");
    assert!(syn_self_bin.exists(), "syn_self binary must be created");

    // 2. Use syn_self to compile examples/control_flow_test.syn
    let self_compile_status = Command::new(&syn_self_bin)
        .arg(&cf_src)
        .arg("-o")
        .arg(&cf_bin)
        .current_dir(&repo_root)
        .status()
        .expect("Failed to invoke syn_self binary on control_flow_test.syn");
    assert!(
        self_compile_status.success(),
        "syn_self failed to compile control_flow_test.syn"
    );
    assert!(cf_bin.exists(), "cf_via_self executable must exist");
    assert!(cf_asm.exists(), "cf_via_self.s assembly must exist");

    // 3. Verify that emitted assembly contains the write-back store instruction (FIND-008 fix)
    let asm_content = fs::read_to_string(&cf_asm).expect("Read emitted assembly");
    assert!(
        asm_content.contains("mov qword ptr [rbp - 8], rax"),
        "Emitted assembly must write updated variable back to [rbp - 8]"
    );

    // 4. Execute cf_via_self with a strict timeout and verify termination with exit code 5
    let start = Instant::now();
    let mut child = Command::new(&cf_bin)
        .spawn()
        .expect("Failed to spawn cf_via_self");

    let timeout = Duration::from_secs(5);
    let mut exit_status = None;
    while start.elapsed() < timeout {
        if let Ok(Some(_)) = child.try_wait() {
            exit_status = child.wait().ok();
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if exit_status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("FATAL: cf_via_self hung in infinite loop (FIND-008 regression)");
    }

    let code = exit_status.unwrap().code();
    assert_eq!(
        code,
        Some(5),
        "cf_via_self must terminate with exit code 5 (15 -> loop to 10 -> loop to 5)"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
