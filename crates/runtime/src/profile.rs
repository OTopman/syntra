use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct FunctionProfile {
    pub name: String,
    pub call_count: u64,
    pub total_duration: Duration,
    pub allocation_count: u64,
}

#[derive(Debug, Clone)]
pub struct HardwareProfile {
    pub os: String,
    pub arch: String,
    pub num_cpus: usize,
}

impl HardwareProfile {
    pub fn detect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            num_cpus: std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LockedFunctionProfile {
    pub name: String,
    pub call_count: u64,
    pub is_hot: bool,
    pub weight_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileLockfile {
    pub version: String,
    pub target_arch: String,
    pub integrity_hash: String,
    pub functions: HashMap<String, LockedFunctionProfile>,
}

#[derive(Debug, Clone)]
pub struct RuntimeProfileData {
    pub total_runtime: Duration,
    pub total_allocations: u64,
    pub total_calls: u64,
    pub functions: HashMap<String, FunctionProfile>,
    pub hardware: HardwareProfile,
}

impl Default for RuntimeProfileData {
    fn default() -> Self {
        Self {
            total_runtime: Duration::default(),
            total_allocations: 0,
            total_calls: 0,
            functions: HashMap::new(),
            hardware: HardwareProfile::detect(),
        }
    }
}

impl RuntimeProfileData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_lockfile(lock: &ProfileLockfile) -> Self {
        let mut data = Self::new();
        for (name, fp) in &lock.functions {
            data.total_calls += fp.call_count;
            data.functions.insert(
                name.clone(),
                FunctionProfile {
                    name: fp.name.clone(),
                    call_count: fp.call_count,
                    total_duration: Duration::from_micros(fp.call_count * 10),
                    allocation_count: 0,
                },
            );
        }
        data
    }

    pub fn is_hot(&self, func_name: &str) -> bool {
        if let Some(fp) = self.functions.get(func_name) {
            if self.total_calls > 0 && (fp.call_count as f64 / self.total_calls as f64) >= 0.15 {
                return true;
            }
            if fp.call_count >= 100 {
                return true;
            }
        }
        false
    }

    pub fn to_synlock(&self) -> String {
        let mut sorted_funcs: Vec<&FunctionProfile> = self.functions.values().collect();
        sorted_funcs.sort_by(|a, b| a.name.cmp(&b.name));

        let total_micros = self.total_runtime.as_micros().max(1) as f64;

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        "profile.synlock".hash(&mut hasher);
        self.hardware.arch.hash(&mut hasher);
        for fp in &sorted_funcs {
            fp.name.hash(&mut hasher);
            fp.call_count.hash(&mut hasher);
            self.is_hot(&fp.name).hash(&mut hasher);
        }
        let integrity_hash = format!("{:016x}", hasher.finish());

        let mut funcs_entries = Vec::new();
        for fp in sorted_funcs {
            let func_micros = fp.total_duration.as_micros() as f64;
            let pct = (func_micros / total_micros) * 100.0;
            let is_hot = self.is_hot(&fp.name);
            funcs_entries.push(format!(
                r#"    "{}": {{ "calls": {}, "is_hot": {}, "weight_pct": {:.1} }}"#,
                fp.name, fp.call_count, is_hot, pct
            ));
        }

        format!(
            r#"{{
  "version": "1.0",
  "integrity_hash": "{}",
  "hardware_arch": "{}",
  "functions": {{
{}
  }}
}}
"#,
            integrity_hash,
            self.hardware.arch,
            funcs_entries.join(",\n")
        )
    }

    pub fn from_synlock(content: &str) -> Option<ProfileLockfile> {
        let mut version = "1.0".to_string();
        let mut integrity_hash = String::new();
        let mut target_arch = String::new();
        let mut functions = HashMap::new();

        let mut in_functions = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("\"version\":") {
                if let Some((_, v)) = trimmed.split_once(':') {
                    version = v
                        .trim()
                        .trim_matches(',')
                        .trim()
                        .trim_matches('"')
                        .to_string();
                }
            } else if trimmed.starts_with("\"integrity_hash\":") {
                if let Some((_, v)) = trimmed.split_once(':') {
                    integrity_hash = v
                        .trim()
                        .trim_matches(',')
                        .trim()
                        .trim_matches('"')
                        .to_string();
                }
            } else if trimmed.starts_with("\"hardware_arch\":") {
                if let Some((_, v)) = trimmed.split_once(':') {
                    target_arch = v
                        .trim()
                        .trim_matches(',')
                        .trim()
                        .trim_matches('"')
                        .to_string();
                }
            } else if trimmed.starts_with("\"functions\":") {
                in_functions = true;
            } else if in_functions {
                if trimmed.starts_with('}') {
                    in_functions = false;
                } else if trimmed.contains(':') && trimmed.contains('{') {
                    if let Some((name_part, rest)) = trimmed.split_once(':') {
                        let name = name_part.trim().trim_matches('"').to_string();
                        let calls = rest
                            .find("\"calls\":")
                            .and_then(|p| {
                                rest[p + 8..]
                                    .split_once(',')
                                    .or_else(|| rest[p + 8..].split_once('}'))
                            })
                            .and_then(|(v, _)| v.trim().parse::<u64>().ok())
                            .unwrap_or(0);

                        let is_hot = rest.contains("\"is_hot\": true");
                        let weight_pct = rest
                            .find("\"weight_pct\":")
                            .and_then(|p| {
                                let after = &rest[p + 13..];
                                let num_str = after.split_whitespace().next().unwrap_or("0");
                                num_str
                                    .trim_matches(',')
                                    .trim_matches('}')
                                    .parse::<f64>()
                                    .ok()
                            })
                            .unwrap_or(0.0);

                        functions.insert(
                            name.clone(),
                            LockedFunctionProfile {
                                name,
                                call_count: calls,
                                is_hot,
                                weight_pct,
                            },
                        );
                    }
                }
            }
        }

        Some(ProfileLockfile {
            version,
            target_arch,
            integrity_hash,
            functions,
        })
    }

    pub fn format_table(&self) -> String {
        let mut out = String::new();
        out.push_str("=========================================================================================\n");
        out.push_str("                       Syntra Adaptive Runtime (SAR) Profile                             \n");
        out.push_str("=========================================================================================\n");
        out.push_str(&format!(
            "• Platform / Arch:   {} ({})\n",
            self.hardware.os, self.hardware.arch
        ));
        out.push_str(&format!(
            "• Logical CPU Cores: {}\n",
            self.hardware.num_cpus
        ));
        out.push_str(&format!(
            "• Total Execution:   {:.2} ms\n",
            self.total_runtime.as_micros() as f64 / 1000.0
        ));
        out.push_str(&format!(
            "• Total Allocations: {}\n",
            self.total_allocations
        ));
        out.push_str(&format!("• Total Calls:       {}\n", self.total_calls));
        out.push_str("-----------------------------------------------------------------------------------------\n");
        out.push_str(
            format!(
                "{:<24} {:<12} {:<16} {:<10} {:<12} {:<8}\n",
                "Function", "Calls", "Time (µs)", "% Time", "Allocs", "PGO Hot"
            )
            .as_str(),
        );
        out.push_str("-----------------------------------------------------------------------------------------\n");

        let mut func_list: Vec<&FunctionProfile> = self.functions.values().collect();
        func_list.sort_by(|a, b| b.total_duration.cmp(&a.total_duration));

        let total_micros = self.total_runtime.as_micros().max(1) as f64;

        for fp in func_list {
            let func_micros = fp.total_duration.as_micros() as f64;
            let pct = (func_micros / total_micros) * 100.0;
            let hot_str = if self.is_hot(&fp.name) {
                "🔥 HOT"
            } else {
                "  -"
            };

            out.push_str(&format!(
                "{:<24} {:<12} {:<16.1} {:<9.1}% {:<12} {:<8}\n",
                fp.name, fp.call_count, func_micros, pct, fp.allocation_count, hot_str
            ));
        }

        out.push_str("=========================================================================================\n");
        out
    }

    pub fn to_json(&self) -> String {
        let mut funcs_json = Vec::new();
        for (name, fp) in &self.functions {
            funcs_json.push(format!(
                r#"    "{}": {{ "calls": {}, "time_us": {}, "allocs": {}, "is_hot": {} }}"#,
                name,
                fp.call_count,
                fp.total_duration.as_micros(),
                fp.allocation_count,
                self.is_hot(name)
            ));
        }

        format!(
            r#"{{
  "total_runtime_us": {},
  "total_allocations": {},
  "total_calls": {},
  "hardware": {{
    "os": "{}",
    "arch": "{}",
    "num_cpus": {}
  }},
  "functions": {{
{}
  }}
}}
"#,
            self.total_runtime.as_micros(),
            self.total_allocations,
            self.total_calls,
            self.hardware.os,
            self.hardware.arch,
            self.hardware.num_cpus,
            funcs_json.join(",\n")
        )
    }
}

pub struct RuntimeProfileCollector {
    pub enabled: bool,
    start_time: Option<Instant>,
    function_calls: HashMap<String, u64>,
    function_times: HashMap<String, Duration>,
    function_allocs: HashMap<String, u64>,
    total_allocations: u64,
    call_stack: Vec<(String, Instant)>,
}

impl Default for RuntimeProfileCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeProfileCollector {
    pub fn new() -> Self {
        Self {
            enabled: false,
            start_time: None,
            function_calls: HashMap::new(),
            function_times: HashMap::new(),
            function_allocs: HashMap::new(),
            total_allocations: 0,
            call_stack: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct SharedProfileCollector(pub Arc<Mutex<RuntimeProfileCollector>>);

impl Default for SharedProfileCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedProfileCollector {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(RuntimeProfileCollector::new())))
    }

    pub fn start(&self) {
        if let Ok(mut c) = self.0.lock() {
            c.enabled = true;
            c.start_time = Some(Instant::now());
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.0.lock().map(|c| c.enabled).unwrap_or(false)
    }

    pub fn record_call_enter(&self, name: &str) {
        if let Ok(mut c) = self.0.lock() {
            if !c.enabled {
                return;
            }
            *c.function_calls.entry(name.to_string()).or_insert(0) += 1;
            c.call_stack.push((name.to_string(), Instant::now()));
        }
    }

    pub fn record_call_exit(&self, name: &str) {
        if let Ok(mut c) = self.0.lock() {
            if !c.enabled {
                return;
            }
            if let Some((stack_name, enter_time)) = c.call_stack.pop() {
                if stack_name == name {
                    let dur = enter_time.elapsed();
                    *c.function_times
                        .entry(name.to_string())
                        .or_insert(Duration::ZERO) += dur;
                }
            }
        }
    }

    pub fn record_alloc(&self, _class_name: &str) {
        if let Ok(mut c) = self.0.lock() {
            if !c.enabled {
                return;
            }
            c.total_allocations += 1;
            let current_func = c.call_stack.last().map(|(f, _)| f.clone());
            if let Some(func) = current_func {
                *c.function_allocs.entry(func).or_insert(0) += 1;
            }
        }
    }

    pub fn finish(&self) -> RuntimeProfileData {
        let c = self.0.lock().unwrap();
        let total_runtime = c.start_time.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
        let mut functions = HashMap::new();
        let mut total_calls = 0;

        for (name, &calls) in &c.function_calls {
            total_calls += calls;
            let total_duration = c
                .function_times
                .get(name)
                .cloned()
                .unwrap_or(Duration::ZERO);
            let allocation_count = c.function_allocs.get(name).cloned().unwrap_or(0);
            functions.insert(
                name.clone(),
                FunctionProfile {
                    name: name.clone(),
                    call_count: calls,
                    total_duration,
                    allocation_count,
                },
            );
        }

        RuntimeProfileData {
            total_runtime,
            total_allocations: c.total_allocations,
            total_calls,
            functions,
            hardware: HardwareProfile::detect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_synlock_roundtrip() {
        let mut functions = HashMap::new();
        functions.insert(
            "calculate".to_string(),
            FunctionProfile {
                name: "calculate".to_string(),
                call_count: 500,
                total_duration: Duration::from_micros(1250),
                allocation_count: 0,
            },
        );
        functions.insert(
            "add".to_string(),
            FunctionProfile {
                name: "add".to_string(),
                call_count: 20,
                total_duration: Duration::from_micros(50),
                allocation_count: 0,
            },
        );

        let data = RuntimeProfileData {
            total_runtime: Duration::from_micros(1500),
            total_allocations: 0,
            total_calls: 520,
            functions,
            hardware: HardwareProfile {
                os: "macos".to_string(),
                arch: "arm64".to_string(),
                num_cpus: 8,
            },
        };

        let synlock = data.to_synlock();
        assert!(synlock.contains("\"version\": \"1.0\""));
        assert!(synlock.contains("\"integrity_hash\":"));
        assert!(synlock.contains("\"calculate\":"));
        assert!(synlock.contains("\"is_hot\": true"));

        let parsed = RuntimeProfileData::from_synlock(&synlock).expect("failed to parse synlock");
        assert_eq!(parsed.version, "1.0");
        assert_eq!(parsed.target_arch, "arm64");
        assert_eq!(parsed.functions.len(), 2);
        assert_eq!(parsed.functions["calculate"].call_count, 500);
        assert!(parsed.functions["calculate"].is_hot);
        assert_eq!(parsed.functions["add"].call_count, 20);
        assert!(!parsed.functions["add"].is_hot);
    }
}
