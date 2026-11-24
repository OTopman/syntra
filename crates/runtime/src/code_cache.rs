use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Cryptographic and semantic identity of a compiled code specialisation (Specification 06 Section 7)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecialisationIdentity {
    pub function_name: String,
    pub strategy_id: String,
    pub air_digest: String,
    pub target_triple: String,
    pub guard_fingerprint: String,
}

impl SpecialisationIdentity {
    pub fn new(
        function_name: &str,
        strategy_id: &str,
        air_digest: &str,
        target_triple: &str,
        guard_fingerprint: &str,
    ) -> Self {
        Self {
            function_name: function_name.to_string(),
            strategy_id: strategy_id.to_string(),
            air_digest: air_digest.to_string(),
            target_triple: target_triple.to_string(),
            guard_fingerprint: guard_fingerprint.to_string(),
        }
    }
}

/// An entry stored in the bounded code cache
#[derive(Debug, Clone)]
pub struct CodeCacheEntry {
    pub identity: SpecialisationIdentity,
    pub bytecode_or_native_size: usize,
    pub invocation_count: u64,
    pub last_access: Instant,
    pub active_pins: usize, // Reference count of frames currently executing this code (RUN-025)
    pub is_retired: bool,
}

impl CodeCacheEntry {
    pub fn new(identity: SpecialisationIdentity, size_bytes: usize) -> Self {
        Self {
            identity,
            bytecode_or_native_size: size_bytes,
            invocation_count: 0,
            last_access: Instant::now(),
            active_pins: 0,
            is_retired: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeCacheError {
    CapacityExceeded,
    EntryPinned(String),
    NotFound(String),
    AlreadyExists(String),
}

/// Thread-Safe Bounded Code Cache (Specification 06 Section 7)
#[derive(Debug)]
pub struct CodeCache {
    entries: HashMap<SpecialisationIdentity, CodeCacheEntry>,
    max_entries: usize,
    max_memory_bytes: usize,
    current_memory_bytes: usize,
}

impl CodeCache {
    pub fn new(max_entries: usize, max_memory_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            max_memory_bytes,
            current_memory_bytes: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn current_memory(&self) -> usize {
        self.current_memory_bytes
    }

    /// Lookup a specialization entry and update its access statistics
    pub fn lookup(&mut self, id: &SpecialisationIdentity) -> Option<&CodeCacheEntry> {
        if let Some(entry) = self.entries.get_mut(id) {
            if !entry.is_retired {
                entry.invocation_count = entry.invocation_count.saturating_add(1);
                entry.last_access = Instant::now();
                return Some(entry);
            }
        }
        None
    }

    /// Pin an entry to protect it from eviction while executing (Specification 06, RUN-025)
    pub fn pin(&mut self, id: &SpecialisationIdentity) -> Result<(), CodeCacheError> {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.active_pins += 1;
            entry.last_access = Instant::now();
            Ok(())
        } else {
            Err(CodeCacheError::NotFound(id.function_name.clone()))
        }
    }

    /// Unpin an entry when execution frame completes
    pub fn unpin(&mut self, id: &SpecialisationIdentity) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.active_pins = entry.active_pins.saturating_sub(1);
        }
    }

    /// Retire an entry so subsequent invocations fall back to baseline
    pub fn retire(&mut self, id: &SpecialisationIdentity) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.is_retired = true;
        }
    }

    /// Inserts a new code specialization, evicting unpinned LRU entries if capacity is reached
    pub fn insert(&mut self, entry: CodeCacheEntry) -> Result<(), CodeCacheError> {
        let size = entry.bytecode_or_native_size;

        // Evict LRU entries if at entry or memory limit
        while (self.entries.len() >= self.max_entries
            || self.current_memory_bytes + size > self.max_memory_bytes)
            && !self.entries.is_empty()
        {
            if self.evict_one_lru().is_none() {
                // All entries are currently pinned (RUN-025)
                return Err(CodeCacheError::CapacityExceeded);
            }
        }

        self.current_memory_bytes += size;
        self.entries.insert(entry.identity.clone(), entry);
        Ok(())
    }

    /// Evicts the least-recently-used entry that has zero active pins (RUN-025)
    fn evict_one_lru(&mut self) -> Option<SpecialisationIdentity> {
        let mut oldest_key: Option<SpecialisationIdentity> = None;
        let mut oldest_time = Instant::now();

        for (id, entry) in &self.entries {
            // Cannot evict code currently in use by any thread (RUN-025)
            if entry.active_pins == 0 && (oldest_key.is_none() || entry.last_access < oldest_time) {
                oldest_time = entry.last_access;
                oldest_key = Some(id.clone());
            }
        }

        if let Some(key) = oldest_key {
            if let Some(removed) = self.entries.remove(&key) {
                self.current_memory_bytes = self
                    .current_memory_bytes
                    .saturating_sub(removed.bytecode_or_native_size);
                return Some(key);
            }
        }
        None
    }
}

pub type SharedCodeCache = Arc<Mutex<CodeCache>>;

pub fn create_shared_code_cache(max_entries: usize, max_bytes: usize) -> SharedCodeCache {
    Arc::new(Mutex::new(CodeCache::new(max_entries, max_bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_cache_lru_and_active_pinning() {
        let mut cache = CodeCache::new(2, 1024);

        let id1 = SpecialisationIdentity::new("fn1", "opt1", "d1", "x86_64", "g1");
        let id2 = SpecialisationIdentity::new("fn2", "opt1", "d2", "x86_64", "g2");
        let id3 = SpecialisationIdentity::new("fn3", "opt1", "d3", "x86_64", "g3");

        assert!(cache.insert(CodeCacheEntry::new(id1.clone(), 100)).is_ok());
        assert!(cache.insert(CodeCacheEntry::new(id2.clone(), 100)).is_ok());

        // Pin id1 so it cannot be evicted
        assert!(cache.pin(&id1).is_ok());

        // Inserting id3 must evict unpinned id2, keeping pinned id1
        assert!(cache.insert(CodeCacheEntry::new(id3.clone(), 100)).is_ok());

        assert!(
            cache.lookup(&id1).is_some(),
            "Pinned entry must NOT be evicted"
        );
        assert!(
            cache.lookup(&id2).is_none(),
            "Unpinned LRU entry must be evicted"
        );
        assert!(cache.lookup(&id3).is_some());

        // Unpin id1 and retire it
        cache.unpin(&id1);
        cache.retire(&id1);
        assert!(
            cache.lookup(&id1).is_none(),
            "Retired entry must not be returned by lookup"
        );
    }
}
