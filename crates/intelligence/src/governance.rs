use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Audit and governance events conforming to Specification 08 Section 4 & 7
#[derive(Debug, Clone)]
pub struct GovernanceAuditEntry {
    pub timestamp_sec: u64,
    pub action: String,
    pub actor: String,
    pub target_digest: String,
    pub details: String,
}

impl GovernanceAuditEntry {
    pub fn new(action: &str, actor: &str, target_digest: &str, details: &str) -> Self {
        let timestamp_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            timestamp_sec,
            action: action.to_string(),
            actor: actor.to_string(),
            target_digest: target_digest.to_string(),
            details: details.to_string(),
        }
    }

    pub fn to_json_line(&self) -> String {
        format!(
            r#"{{"ts":{},"action":"{}","actor":"{}","digest":"{}","details":"{}"}}"#,
            self.timestamp_sec,
            self.action,
            self.actor,
            self.target_digest,
            self.details.replace('"', "\\\"")
        )
    }

    pub fn from_json_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            return None;
        }

        let extract_field = |key: &str| -> Option<String> {
            let pattern = format!("\"{key}\":");
            let pos = trimmed.find(&pattern)?;
            let rest = &trimmed[pos + pattern.len()..];
            let rest_trimmed = rest.trim_start();
            if let Some(stripped) = rest_trimmed.strip_prefix('"') {
                let closing = stripped.find('"')?;
                Some(stripped[..closing].to_string())
            } else {
                let end = rest_trimmed.find([',', '}']).unwrap_or(rest_trimmed.len());
                Some(rest_trimmed[..end].trim().to_string())
            }
        };

        let timestamp_sec = extract_field("ts")?.parse().ok()?;
        let action = extract_field("action")?;
        let actor = extract_field("actor").unwrap_or_else(|| "system".to_string());
        let target_digest = extract_field("digest").unwrap_or_default();
        let details = extract_field("details").unwrap_or_default();

        Some(Self {
            timestamp_sec,
            action,
            actor,
            target_digest,
            details,
        })
    }
}

/// Persistent audit trail for dataset mutations, model promotion, rollback, and revocation (LEARN-005)
#[derive(Debug, Default)]
pub struct GovernanceAuditLog {
    pub entries: Vec<GovernanceAuditEntry>,
}

impl GovernanceAuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, entry: GovernanceAuditEntry, path: Option<&Path>) {
        if let Some(p) = path {
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(p) {
                let _ = writeln!(f, "{}", entry.to_json_line());
            }
        }
        self.entries.push(entry);
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read audit log {}: {}", path.display(), e))?;

        let mut entries = Vec::new();
        for line in content.lines() {
            if let Some(entry) = GovernanceAuditEntry::from_json_line(line) {
                entries.push(entry);
            }
        }
        Ok(Self { entries })
    }
}

/// Reason for segregating a record to the quarantine dataset (Specification 08, LEARN-002)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuarantineReason {
    SchemaMismatch(String),
    PoisonedData(String),
    ContradictoryVerifierState(String),
    MissingMetadata(String),
    NonPositiveLatency,
    ExtremeAnomalySpeedup(String),
}

impl std::fmt::Display for QuarantineReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemaMismatch(s) => write!(f, "Schema mismatch: {s}"),
            Self::PoisonedData(s) => write!(f, "Poisoned data detected: {s}"),
            Self::ContradictoryVerifierState(s) => write!(f, "Contradictory verifier state: {s}"),
            Self::MissingMetadata(s) => write!(f, "Missing metadata: {s}"),
            Self::NonPositiveLatency => write!(f, "Non-positive latency measured"),
            Self::ExtremeAnomalySpeedup(s) => write!(f, "Extreme anomalous speedup: {s}"),
        }
    }
}
