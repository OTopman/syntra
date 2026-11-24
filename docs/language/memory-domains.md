# Memory Domains & Safety

Syntra guarantees memory safety through explicit memory domains and linear ownership semantics enforced at compile time via the Intermediate Representation verifier (`syntra_air`).

---

## 🛡️ Invariant Specifications (MEM-001..004)

Syntra eliminates entire categories of systems vulnerabilities without garbage collection pauses:

* **MEM-001 (Zero-Leak Ownership):** Every allocated resource has a single unique owner. When the owner scope exits, resources are deallocated immediately and deterministically.
* **MEM-002 (Domain Isolation):** Pointers allocated in one memory domain cannot cross boundaries into another domain without explicit transfer or cloning.
* **MEM-003 (No Use-After-Free):** The compiler tracks the lifetime of borrowed references. Borrowed references cannot outlive their parent allocation.
* **MEM-004 (Linear Exclusive Mutation):** A variable may have either multiple immutable borrows or exactly one mutable borrow at any given time.

---

## 🏛️ Memory Domain Types

Configured in [`Package.syn`](file:///Users/Topman/Documents/Projects/aether/Package.syn) under `build { memory_domains = "..." }`:

### 1. Strict Domains (`memory_domains = "strict"`)
Default mode for high-security, mission-critical systems:
* Every memory allocation, move, and borrow is verified by the AIR invariant checker at compile time.
* Prohibits cyclic references unless wrapped in explicit weak handles.
* Zero runtime garbage collection overhead.

### 2. Permissive Domains (`memory_domains = "permissive"`)
Designed for rapid prototyping and legacy script migration:
* Allows dynamic reference graphs.
* Cross-domain boundary safety is enforced with lightweight runtime guard assertions.

### 3. Arena Domains (`memory_domains = "arena"`)
Optimized for high-throughput batch compilers and game loops:
* Objects are allocated into high-speed bump-allocated memory arenas.
* Individual objects do not incur deallocation overhead; the entire arena is reclaimed in a single operation upon loop or frame completion.

```syntra
// High-throughput arena allocation example
func process_batch(int count): void {
    let arena = new ArenaDomain();
    for i in 0..count {
        let node = arena.allocate(new Node(i));
        node.compute();
    }
    // Entire arena memory reclaimed instantly here
}
```
