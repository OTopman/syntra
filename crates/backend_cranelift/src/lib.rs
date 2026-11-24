//! # Syntra Cranelift Native Backend
//!
//! Compiles SSA AIR intermediate representation modules directly into machine code objects
//! and links standalone native binaries using Cranelift code generator.
//! Supports x86-64 and aarch64 (Apple Silicon / Linux ARM64).

pub mod compiler;

pub use compiler::{
    compile_module_to_object, compile_module_to_object_target, link_executable,
    link_executable_target, BackendError,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    fn temp_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("syntra_backend_tests").join(name);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_native_compile_and_link_return_code() {
        let code = r#"
func main(): int {
    int a = 20;
    int b = 22;
    return a + b;
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");

        let dir = temp_test_dir("arithmetic");
        let obj_path = dir.join("test.o");
        let bin_path = dir.join("test_bin");

        compile_module_to_object(&module, &obj_path).expect("Object compilation failed");
        assert!(obj_path.exists());

        link_executable(&obj_path, &bin_path).expect("Link failed");
        assert!(bin_path.exists());

        let output = Command::new(&bin_path)
            .output()
            .expect("Failed to execute binary");
        assert_eq!(output.status.code(), Some(42));

        let _ = fs::remove_file(obj_path);
        let _ = fs::remove_file(bin_path);
    }

    #[test]
    fn test_native_compile_void_main() {
        let code = r#"
func main() {
    int x = 10;
    int y = x * 2;
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");

        let dir = temp_test_dir("void_main");
        let obj_path = dir.join("void_test.o");
        let bin_path = dir.join("void_test_bin");

        compile_module_to_object(&module, &obj_path).expect("Object compilation failed");
        link_executable(&obj_path, &bin_path).expect("Link failed");

        let output = Command::new(&bin_path)
            .output()
            .expect("Failed to execute binary");
        assert_eq!(output.status.code(), Some(0));

        let _ = fs::remove_file(obj_path);
        let _ = fs::remove_file(bin_path);
    }

    #[test]
    fn test_native_compile_float_arithmetic() {
        let code = r#"
func main(): int {
    float a = 12.5;
    float b = 2.5;
    float c = a / b;
    if (c == 5.0) {
        return 42;
    }
    return 1;
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");

        let dir = temp_test_dir("float_math");
        let obj_path = dir.join("float_test.o");
        let bin_path = dir.join("float_test_bin");

        compile_module_to_object(&module, &obj_path).expect("Object compilation failed");
        link_executable(&obj_path, &bin_path).expect("Link failed");

        let output = Command::new(&bin_path)
            .output()
            .expect("Failed to execute binary");
        assert_eq!(output.status.code(), Some(42));

        let _ = fs::remove_file(obj_path);
        let _ = fs::remove_file(bin_path);
    }

    #[test]
    fn test_native_compile_function_calls_and_recursion() {
        let code = r#"
func fib(int n): int {
    if (n <= 1) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

func main(): int {
    return fib(6);
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");

        let dir = temp_test_dir("recursion");
        let obj_path = dir.join("fib_test.o");
        let bin_path = dir.join("fib_test_bin");

        compile_module_to_object(&module, &obj_path).expect("Object compilation failed");
        link_executable(&obj_path, &bin_path).expect("Link failed");

        let output = Command::new(&bin_path)
            .output()
            .expect("Failed to execute binary");
        // fib(6) = 8
        assert_eq!(output.status.code(), Some(8));

        let _ = fs::remove_file(obj_path);
        let _ = fs::remove_file(bin_path);
    }

    #[test]
    fn test_native_cross_target_isa_arm64_and_x86_64() {
        let code = r#"
func main(): int {
    return 100;
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");

        let dir = temp_test_dir("cross_isa");
        let obj_x86 = dir.join("test_x86.o");
        let obj_arm = dir.join("test_arm.o");

        compile_module_to_object_target(&module, &obj_x86, Some("x86_64-apple-darwin"))
            .expect("x86_64 compilation failed");
        assert!(obj_x86.exists());

        compile_module_to_object_target(&module, &obj_arm, Some("aarch64-apple-darwin"))
            .expect("arm64 compilation failed");
        assert!(obj_arm.exists());

        let _ = fs::remove_file(obj_x86);
        let _ = fs::remove_file(obj_arm);
    }

    #[test]
    fn test_native_channel_select_concurrency() {
        let code = r#"
fn worker(Channel ch): int {
    Task.sleep(20);
    ch.send(77);
    return 1;
}

fn main(): int {
    Channel ch1 = new Channel();
    Channel ch2 = new Channel();
    Task.spawn(worker, ch2);
    Channel[] chs = [ch1, ch2];
    int idx = Channel.select(chs);
    if (idx != 1) {
        return 101;
    }
    int val = ch2.recv();
    if (val != 77) {
        return 102;
    }
    return 42;
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");

        let dir = temp_test_dir("channel_select");
        let obj_path = dir.join("chan_sel.o");
        let bin_path = dir.join("chan_sel_bin");

        compile_module_to_object(&module, &obj_path).expect("Object compilation failed");
        link_executable(&obj_path, &bin_path).expect("Link failed");

        let output = Command::new(&bin_path)
            .output()
            .expect("Failed to execute binary");
        assert_eq!(output.status.code(), Some(42));

        let _ = fs::remove_file(obj_path);
        let _ = fs::remove_file(bin_path);
    }

    #[test]
    fn test_native_gc_reclamation_loop() {
        let code = r#"
class Node {
    int value;
    Node next;
}

fn main(): int {
    int i = 0;
    while (i < 500) {
        Node n = new Node();
        n.value = i;
        i = i + 1;
    }
    return 42;
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");

        let dir = temp_test_dir("gc_loop");
        let obj_path = dir.join("gc_loop.o");
        let bin_path = dir.join("gc_loop_bin");

        compile_module_to_object(&module, &obj_path).expect("Object compilation failed");
        link_executable(&obj_path, &bin_path).expect("Link failed");

        let output = Command::new(&bin_path)
            .output()
            .expect("Failed to execute binary");
        assert_eq!(output.status.code(), Some(42));

        let _ = fs::remove_file(obj_path);
        let _ = fs::remove_file(bin_path);
    }

    #[test]
    fn test_native_runtime_gc_direct_c_api() {
        let test_c_code = r#"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>

void* syntra_alloc(int64_t size);
void syntra_free(void* ptr);
void syntra_gc_collect(void);
int64_t syntra_memory_allocated_bytes(void);
int64_t syntra_gc_total_collections(void);
void syntra_gc_set_threshold(int64_t threshold_bytes);

__attribute__((noinline))
void allocate_garbage(void) {
    for (int i = 0; i < 100; i++) {
        volatile void* p = syntra_alloc(64);
        (void)p;
    }
}

int main(void) {
    syntra_gc_set_threshold(4096);
    int64_t initial = syntra_memory_allocated_bytes();
    assert(initial == 0);

    // Allocate 100 unreferenced objects in a separate frame
    allocate_garbage();
    int64_t during = syntra_memory_allocated_bytes();
    assert(during > 0);

    // Collect garbage - unreferenced objects should be swept
    syntra_gc_collect();
    int64_t after = syntra_memory_allocated_bytes();
    assert(after < during);

    // Test explicit syntra_free
    void* explicit_obj = syntra_alloc(128);
    int64_t with_explicit = syntra_memory_allocated_bytes();
    syntra_free(explicit_obj);
    int64_t after_free = syntra_memory_allocated_bytes();
    assert(after_free < with_explicit);

    return 42;
}
"#;
        let dir = temp_test_dir("gc_c_api");
        let src_c = dir.join("test_gc.c");
        let bin = dir.join("test_gc_bin");
        fs::write(&src_c, test_c_code).unwrap();

        let rt_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../runtime/native/syntra_rt.c");
        let status = Command::new("cc")
            .arg(&src_c)
            .arg(&rt_path)
            .arg("-O0")
            .arg("-o")
            .arg(&bin)
            .status()
            .expect("Failed to compile C GC test");
        assert!(status.success());

        let out = Command::new(&bin)
            .output()
            .expect("Failed to run C GC test");
        if out.status.code() != Some(42) {
            eprintln!("STDOUT: {}", String::from_utf8_lossy(&out.stdout));
            eprintln!("STDERR: {}", String::from_utf8_lossy(&out.stderr));
        }
        assert_eq!(out.status.code(), Some(42));

        let _ = fs::remove_file(src_c);
        let _ = fs::remove_file(bin);
    }

    #[test]
    fn test_native_closure_capture_and_system_time_parity() {
        let code = r#"
class MathHelper {
    func doubleVal(int x): int {
        return x * 2;
    }
}

func main(): int {
    MathHelper helper = new MathHelper();

    // Lambda defined in scope with outer typed variable present
    any f = (int step) => {
        return step * 2;
    };
    int res = f(21);

    int t = System.currentTimeMillis();
    if (t > 0 && res == 42) {
        return 42;
    }
    return 1;
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");

        let interp_res = syntra_runtime::run(&prog).expect("Interpreter failed");
        assert_eq!(interp_res, syntra_runtime::Value::Int(42));

        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");
        let dir = temp_test_dir("closure_parity");
        let obj_path = dir.join("test_closure.o");
        let bin_path = dir.join("test_closure_bin");

        compile_module_to_object(&module, &obj_path).expect("Object compilation failed");
        link_executable(&obj_path, &bin_path).expect("Link failed");

        let output = Command::new(&bin_path)
            .output()
            .expect("Failed to execute binary");
        assert_eq!(output.status.code(), Some(42));

        let _ = fs::remove_file(obj_path);
        let _ = fs::remove_file(bin_path);
    }
}
