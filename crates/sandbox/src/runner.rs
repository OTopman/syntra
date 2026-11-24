use crate::candidate::Candidate;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub timed_out: bool,
}

#[derive(Debug, Clone)]
pub struct CompilationOutput {
    pub binary_path: PathBuf,
    pub binary_size_bytes: u64,
    pub compile_duration: Duration,
}

pub struct SandboxRunner {
    sandbox_base: PathBuf,
    run_id: String,
    timeout: Duration,
    max_output_bytes: usize,
    env_vars: Vec<(String, String)>,
}

impl SandboxRunner {
    pub fn new(run_id: String) -> Self {
        let sandbox_base = std::env::temp_dir().join("syntra_sandbox").join(&run_id);
        Self {
            sandbox_base,
            run_id,
            timeout: Duration::from_secs(5),
            max_output_bytes: 1024 * 1024, // 1 MB
            env_vars: Vec::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_output_bytes(mut self, max: usize) -> Self {
        self.max_output_bytes = max;
        self
    }

    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.env_vars.push((key, value));
        self
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn max_output_bytes(&self) -> usize {
        self.max_output_bytes
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn sandbox_base(&self) -> &Path {
        &self.sandbox_base
    }

    /// Prepares an isolated directory for the candidate
    pub fn candidate_dir(&self, candidate_id: &str) -> PathBuf {
        self.sandbox_base.join(candidate_id)
    }

    /// Compiles a candidate's AIR module to an isolated native binary
    pub fn compile_candidate(&self, candidate: &Candidate) -> Result<CompilationOutput, String> {
        let dir = self.candidate_dir(&candidate.id);
        fs::create_dir_all(&dir).map_err(|e| {
            format!(
                "Failed to create candidate sandbox dir '{}': {}",
                dir.display(),
                e
            )
        })?;

        let obj_path = dir.join("candidate.o");
        let bin_path = dir.join("candidate_bin");

        let t0 = Instant::now();

        // 1. Compile AIR to machine code object file via Cranelift backend
        syntra_backend_cranelift::compile_module_to_object(&candidate.air_module, &obj_path)
            .map_err(|e| {
                format!(
                    "Cranelift object compilation failed for {}: {}",
                    candidate.id, e
                )
            })?;

        // 2. Link executable with system linker and runtime
        syntra_backend_cranelift::link_executable(&obj_path, &bin_path)
            .map_err(|e| format!("Linking executable failed for {}: {}", candidate.id, e))?;

        let compile_duration = t0.elapsed();

        // Clean up temporary .o
        let _ = fs::remove_file(&obj_path);

        let binary_size_bytes = fs::metadata(&bin_path).map(|m| m.len()).unwrap_or(0);

        Ok(CompilationOutput {
            binary_path: bin_path,
            binary_size_bytes,
            compile_duration,
        })
    }

    /// Executes the candidate binary in sandbox with arguments, strict timeouts, environment isolation,
    /// and output bounding.
    pub fn execute_binary(&self, binary_path: &Path, args: &[String]) -> Result<RunOutput, String> {
        if !binary_path.exists() {
            return Err(format!("Binary does not exist: {}", binary_path.display()));
        }

        let parent_dir = binary_path.parent().unwrap_or(&self.sandbox_base);

        // Sanitize environment: clear all host variables, only provide minimal safe keys
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

        let t0 = Instant::now();
        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to spawn sandboxed binary '{}': {}",
                binary_path.display(),
                e
            )
        })?;

        // Read stdout and stderr in background threads with bounded size limit to avoid pipe deadlocks
        let mut stdout_handle = child.stdout.take();
        let mut stderr_handle = child.stderr.take();
        let max_output = self.max_output_bytes;

        let stdout_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut stream) = stdout_handle.take() {
                let mut limited = (&mut stream).take(max_output as u64);
                let _ = limited.read_to_end(&mut buf);
            }
            buf
        });

        let stderr_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut stream) = stderr_handle.take() {
                let mut limited = (&mut stream).take(max_output as u64);
                let _ = limited.read_to_end(&mut buf);
            }
            buf
        });

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

        let duration = t0.elapsed();
        let stdout_bytes = stdout_thread.join().unwrap_or_default();
        let stderr_bytes = stderr_thread.join().unwrap_or_default();

        let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
        let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

        Ok(RunOutput {
            stdout,
            stderr,
            exit_code,
            duration,
            timed_out,
        })
    }

    /// Clean up the entire sandbox directory for this run
    pub fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.sandbox_base);
    }
}

impl Drop for SandboxRunner {
    fn drop(&mut self) {
        self.cleanup();
    }
}
