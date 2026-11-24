//! # Syntra Runtime Engine
//!
//! The Syntra execution engine provides:
//! - Multi-paradigm AST tree-walk interpreter with scoped lexical environments
//! - Self-Adapting Runtime (SAR) dynamic adaptation governor and telemetry
//! - Deoptimization and on-stack replacement (OSR) state reconstruction
//! - Bounded least-recently-used (LRU) specialized code cache
//! - Dynamic pre-condition guards and invalidation callbacks
//! - Built-in zero-copy RFC 6455 WebSocket frame encoding and streaming
//! - Comprehensive profile-guided optimization (PGO) telemetry recorders

pub mod code_cache;
pub mod debug;
pub mod deopt;
pub mod eval;
pub mod guards;
pub mod profile;
pub mod sar;
pub mod value;
pub mod websocket;

pub use code_cache::{
    CodeCache, CodeCacheEntry, CodeCacheError, SharedCodeCache, SpecialisationIdentity,
};
pub use debug::{
    create_debug_channel, DebugCommand, DebugEvent, DebugHandle, DebugHookState, PauseReason,
    StackFrame, StepMode,
};
pub use deopt::{ContinuationFrame, DeoptEvent, DeoptManager, DeoptReason};
pub use eval::{Interpreter, RuntimeError};
pub use guards::{DynamicGuard, GuardEvaluationResult, GuardManager, TypeTag};
pub use profile::{
    FunctionProfile, HardwareProfile, LockedFunctionProfile, ProfileLockfile,
    RuntimeProfileCollector, RuntimeProfileData, SharedProfileCollector,
};
pub use sar::{AdaptationAction, SarEngine, SarTelemetry, SharedSarEngine, SpecialisationRequest};
pub use value::{Instance, Value};

use syntra_syntax::ast::Program;

/// Executes a parsed Syntra AST program without command-line arguments.
///
/// # Arguments
/// * `program` - The parsed AST program root.
///
/// # Returns
/// * `Ok(Value)` - The termination value returned by `main()`.
/// * `Err(RuntimeError)` - Any runtime evaluation or type violation error.
pub fn run(program: &Program) -> Result<Value, RuntimeError> {
    run_with_args(program, Vec::new())
}

/// Executes a parsed Syntra AST program passing command-line arguments accessible via `System.args()`.
///
/// # Arguments
/// * `program` - The parsed AST program root.
/// * `args` - Array of argument strings passed to the runtime.
///
/// # Returns
/// * `Ok(Value)` - The termination value returned by `main()`.
/// * `Err(RuntimeError)` - Any runtime evaluation or type violation error.
pub fn run_with_args(program: &Program, args: Vec<String>) -> Result<Value, RuntimeError> {
    let mut interp = Interpreter::new();
    interp.set_args(args);
    interp.run_program(program)
}

/// Executes a parsed Syntra program while recording runtime execution telemetry and hardware profiles.
///
/// # Arguments
/// * `program` - The parsed AST program root.
/// * `args` - Command-line argument strings.
///
/// # Returns
/// * `Ok((Value, RuntimeProfileData))` - The program exit value and the gathered profile metrics.
/// * `Err(RuntimeError)` - Any runtime evaluation or type violation error.
pub fn run_with_profiling(
    program: &Program,
    args: Vec<String>,
) -> Result<(Value, RuntimeProfileData), RuntimeError> {
    let mut interp = Interpreter::new();
    interp.set_args(args);
    interp.profiler.start();
    let val = interp.run_program(program)?;
    let profile = interp.profiler.finish();
    Ok((val, profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_array_sum_and_methods() {
        let src = r#"
        class Box {
            public int value;
            new(int v) { this.value = v; }
            public func get_val(): int { return this.value; }
        }

        func main(): int {
            Box[] boxes = [new Box(10), new Box(20), new Box(30)];
            int sum = 0;
            for b in boxes {
                sum = sum + b.get_val();
            }
            return sum;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        let res = run(&prog).expect("Runtime error");
        assert_eq!(res, Value::Int(60));
    }

    #[test]
    fn test_repl_eval_line() {
        let mut interp = Interpreter::new();
        let val1 = interp.eval_line("10 + 20 * 3").expect("eval error");
        assert_eq!(val1, Some(Value::Int(70)));

        interp.eval_line("int a = 15;").expect("decl error");
        let val2 = interp.eval_line("a * 2").expect("eval error");
        assert_eq!(val2, Some(Value::Int(30)));
    }

    #[test]
    fn test_math_namespace_multi_args() {
        let mut interp = Interpreter::new();
        let val1 = interp
            .eval_line("Math.min(50, 10, 80, 5)")
            .expect("eval error");
        assert_eq!(val1, Some(Value::Int(5)));

        let val2 = interp
            .eval_line("Math.max(50, 10, 80, 5)")
            .expect("eval error");
        assert_eq!(val2, Some(Value::Int(80)));

        let val3 = interp
            .eval_line("Math.min([100, 25, 999])")
            .expect("eval error");
        assert_eq!(val3, Some(Value::Int(25)));

        let val4 = interp
            .eval_line("Math.clamp(150, 0, 100)")
            .expect("eval error");
        assert_eq!(val4, Some(Value::Int(100)));
    }

    #[test]
    fn test_string_operations_and_chaining() {
        let mut interp = Interpreter::new();
        let val1 = interp
            .eval_line(r#""  Syntra  ".trim().toUpper()"#)
            .expect("eval error");
        assert_eq!(val1, Some(Value::String("SYNTRA".to_string())));

        let val2 = interp
            .eval_line(r#""hello world".contains("world")"#)
            .expect("eval error");
        assert_eq!(val2, Some(Value::Bool(true)));

        let val3 = interp
            .eval_line(r#""hello world".indexOf("world")"#)
            .expect("eval error");
        assert_eq!(val3, Some(Value::Int(6)));

        let val4 = interp
            .eval_line(r#""a,b,c".split(",")"#)
            .expect("eval error");
        if let Some(Value::Array(arr)) = val4 {
            assert_eq!(arr.lock().unwrap().len(), 3);
        } else {
            panic!("Expected array from split");
        }
    }

    #[test]
    fn test_file_io_runtime_and_oop() {
        let path = "/tmp/syntra_unit_test.txt";
        let src = format!(
            r#"
        func main(): int {{
            string path = "{path}";
            File.writeText(path, "Hello Unit Test\nSecond Line");
            bool ex = File.exists(path);
            string content = File.readText(path);
            int sz = File.size(path);
            File.delete(path);

            File f = new File(path);
            f.writeText("OOP Unit Test");
            string oop_content = f.readText();
            f.delete();

            if (ex && sz > 0 && oop_content == "OOP Unit Test") {{
                return 42;
            }}
            return 0;
        }}
        "#
        );

        let prog = syntra_syntax::parse(&src).expect("Parse error");
        let res = run(&prog).expect("Runtime error");
        assert_eq!(res, Value::Int(42));
    }

    #[test]
    fn test_system_and_error_handling_runtime() {
        let src = r#"
        func main(): int {
            int a = 10;
            int b = 20;
            string s = $"{a} + {b} = {a + b}";

            string caught = "";
            int finally_ran = 0;
            try {
                throw "error 101";
            } catch (err) {
                caught = err;
            } finally {
                finally_ran = 1;
            }

            int now = System.timeMs();
            string[] args = System.args();

            if (s == "10 + 20 = 30" && caught == "error 101" && finally_ran == 1 && now > 0 && args.len() == 2) {
                return 77;
            }
            return 0;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        let res = run_with_args(&prog, vec!["arg1".to_string(), "arg2".to_string()])
            .expect("Runtime error");
        assert_eq!(res, Value::Int(77));
    }

    #[test]
    fn test_maps_json_and_functional_runtime() {
        let src = r#"
        class Box<T> {
            T value;
            constructor(T value) {
                this.value = value;
            }
            func get(): T {
                return this.value;
            }
        }

        func main(): int {
            Map<string, int> m = new Map<string, int>();
            m.set("alpha", 10);
            m.set("beta", 20);
            m["gamma"] = 30;

            int a = m.get("alpha");
            int b = m["beta"];
            int c = m.get("gamma");
            int sz = m.len();

            bool has_b = m.has("beta");
            m.remove("beta");
            bool has_b_after = m.has("beta");

            string json = "{\"greeting\": \"hello\", \"count\": 42}";
            Map<string, string> parsed = Json.parse(json);
            string g = parsed.get("greeting");

            int[] numbers = [1, 2, 3, 4, 5];
            int[] doubled = numbers.map(x => x * 2);
            int[] filtered = doubled.filter(x => x > 4);
            int total = filtered.reduce((acc, x) => acc + x, 0);

            Box<int> boxInst = new Box<int>(100);
            int boxedVal = boxInst.get();

            if (a == 10 && b == 20 && c == 30 && sz == 3 && has_b && has_b_after == false && g == "hello" && total == 24 && boxedVal == 100) {
                return 99;
            }
            return 0;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        let res = run(&prog).expect("Runtime error");
        assert_eq!(res, Value::Int(99));
    }

    #[test]
    fn test_concurrency_tasks_and_channels_runtime() {
        let src = r#"
        fn worker(int x): int {
            Task.sleep(10);
            return x * 2;
        }

        func main(): int {
            Task t = Task.spawn(worker, 21);
            int joined = t.join();

            Channel ch = new Channel();
            ch.send(100);
            ch.send(200);
            int v1 = ch.recv();
            int v2 = ch.recv();
            int sz = ch.len();

            if (joined == 42 && v1 == 100 && v2 == 200 && sz == 0) {
                return 777;
            }
            return 0;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        let res = run(&prog).expect("Runtime error");
        assert_eq!(res, Value::Int(777));
    }
}
