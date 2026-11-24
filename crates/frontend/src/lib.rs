//! # Syntra Frontend Semantic Analysis Engine
//!
//! Provides static semantics, bidirectional type checking, lexical symbol resolution,
//! and recursive multi-file package and dependency loading.

pub mod checker;
pub mod loader;
pub mod symbols;
pub mod types;

pub use checker::TypeChecker;
pub use loader::{
    find_package_manifest, find_syntra_toml, resolve_git_dependency, ModuleLoader,
    PackageDependency, PackageManifest,
};
pub use symbols::*;
pub use types::Type;

use syntra_diagnostics::Diagnostic;
use syntra_syntax::ast::Program;

/// Validates the semantic correctness and static types of a parsed Syntra AST program.
///
/// # Arguments
/// * `program` - The parsed AST program root.
///
/// # Returns
/// * `Ok(())` - Program passed all static analysis checks.
/// * `Err(Vec<Diagnostic>)` - List of semantic diagnostics and type violation errors.
pub fn check(program: &Program) -> Result<(), Vec<Diagnostic>> {
    let mut checker = TypeChecker::new();
    checker.check_program(program)
}

/// Type checks a Syntra AST program and produces the populated global symbol table.
///
/// # Arguments
/// * `program` - The parsed AST program root.
///
/// # Returns
/// * `Ok(SymbolTable)` - Global symbol table with classes, methods, and functions.
/// * `Err(Vec<Diagnostic>)` - List of semantic diagnostics if checking failed.
pub fn check_and_get_symbols(program: &Program) -> Result<SymbolTable, Vec<Diagnostic>> {
    let mut checker = TypeChecker::new();
    checker.check_program(program)?;
    Ok(checker.into_symbols())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_checking_valid_program() {
        let src = r#"
        class Hero {
            public string name;
            public int level;
            new(string name, int level) {
                this.name = name;
                this.level = level;
            }
        }

        func main(): int {
            Hero[] party = [new Hero("Aria", 10), new Hero("Finn", 12)];
            int total = 0;
            for (int i = 0; i < party.length; i = i + 1) {
                total = total + party[i].level;
            }
            return total;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        if let Err(diags) = check(&prog) {
            for d in &diags {
                eprintln!("Type check diag: {d:?}");
            }
            panic!("Type check failed: {} errors", diags.len());
        }
    }

    #[test]
    fn test_type_checking_mismatch_fails() {
        let src = r#"
        func main(): int {
            int x = "not an int";
            return x;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        assert!(check(&prog).is_err());
    }

    #[test]
    fn test_string_methods_type_check() {
        let src = r#"
        func main(): int {
            string s = "Syntra";
            int len = s.length;
            bool c = s.contains("eth");
            string upper = s.toUpper();
            string[] parts = s.split("e");
            string rejoined = parts.join("-");
            return len;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        if let Err(diags) = check(&prog) {
            for d in &diags {
                eprintln!("Type check diag: {d:?}");
            }
            panic!("Type check failed: {} errors", diags.len());
        }
    }

    #[test]
    fn test_module_loader_resolves_imports() {
        let test_path = std::path::PathBuf::from("examples/multi_file_test.syn");
        if test_path.exists() {
            let mut loader = ModuleLoader::new();
            let prog = loader
                .load_program(&test_path)
                .expect("Failed to load program");
            assert!(check(&prog).is_ok());
        }
    }

    #[test]
    fn test_file_io_type_check() {
        let src = r#"
        func main(): int {
            string path = "/tmp/test.txt";
            bool w = File.writeText(path, "content");
            string s = File.readText(path);
            bool e = File.exists(path);
            int sz = File.size(path);
            string[] lines = File.readLines(path);
            bool d = File.delete(path);

            File f = new File("/tmp/oop.txt");
            bool fe = f.exists();
            bool fw = f.writeText("oop");
            string fr = f.readText();
            string fp = f.getPath();
            string fp2 = f.path;
            bool fd = f.delete();
            return sz;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        if let Err(diags) = check(&prog) {
            for d in &diags {
                eprintln!("Type check diag: {d:?}");
            }
            panic!("Type check failed: {} errors", diags.len());
        }
    }

    #[test]
    fn test_system_and_error_handling_type_check() {
        let src = r#"
        func main(): int {
            string[] args = System.args();
            string env = System.getEnv("PATH");
            int now = System.timeMs();
            string user = "Syntra";
            string greeting = $"Hello {user}, time is {now}";

            try {
                throw "runtime problem";
            } catch (string err) {
                greeting = err;
            } finally {
                now = 0;
            }
            return now;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        if let Err(diags) = check(&prog) {
            for d in &diags {
                eprintln!("Type check diag: {d:?}");
            }
            panic!("Type check failed: {} errors", diags.len());
        }
    }

    #[test]
    fn test_maps_and_json_and_lambdas_type_check() {
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
            m.set("apple", 100);
            m.set("banana", 200);
            int a = m.get("apple");
            bool h = m.has("banana");
            int sz = m.len();
            string[] k = m.keys();

            string jsonStr = "{\"id\": 1, \"name\": \"Syntra\"}";
            Map<string, string> parsed = Json.parse(jsonStr);
            string encoded = Json.stringify(m);

            int[] nums = [1, 2, 3, 4, 5];
            int[] doubled = nums.map((int x) => x * 2);
            int[] evens = nums.filter(x => x > 2);
            int sum = nums.reduce((int acc, int x) => acc + x, 0);

            Box<int> b = new Box<int>(42);
            int val = b.get();

            return val;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        if let Err(diags) = check(&prog) {
            for d in &diags {
                eprintln!("Type check diag: {d:?}");
            }
            panic!("Type check failed: {} errors", diags.len());
        }
    }

    #[test]
    fn test_async_await_type_check_valid() {
        let src = r#"
        async fn get_count(): int {
            return 42;
        }

        async fn get_name(): string {
            return "Syntra";
        }

        fn main(): void {
            int c = await get_count();
            string n = await get_name();
            Task t = get_count();
            int c2 = await t;
            int sum = await get_count() + 10;
        }
        "#;
        let prog = syntra_syntax::parse(src).expect("Parse error");
        assert!(
            check(&prog).is_ok(),
            "Valid async/await types must pass type check"
        );
    }

    #[test]
    fn test_async_await_type_mismatch_fails() {
        let src_assign_mismatch = r#"
        async fn get_count(): int {
            return 42;
        }

        fn main(): void {
            string s = await get_count();
        }
        "#;
        let prog1 = syntra_syntax::parse(src_assign_mismatch).expect("Parse error");
        assert!(
            check(&prog1).is_err(),
            "Assigning await int to string must fail type check"
        );

        let src_await_non_task = r#"
        fn main(): void {
            int x = await 42;
        }
        "#;
        let prog2 = syntra_syntax::parse(src_await_non_task).expect("Parse error");
        let res2 = check(&prog2);
        assert!(res2.is_err(), "Awaiting non-Task int must fail type check");
        let errs = res2.unwrap_err();
        assert!(errs
            .iter()
            .any(|d| d.message.contains("'await' expression expects a Task")));
    }

    #[test]
    fn test_http_response_set_header_type_check() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut loader =
            loader::ModuleLoader::new().with_search_paths(vec![repo_root.join("stdlib")]);
        let http_example = repo_root.join("examples/http_server_test.syn");
        let prog = loader
            .load_program(&http_example)
            .expect("Load http_server_test.syn");
        let check_res = check(&prog);
        assert!(
            check_res.is_ok(),
            "http_server_test with setHeader should typecheck cleanly"
        );
    }
}
