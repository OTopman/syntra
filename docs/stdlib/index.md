# Standard Library Reference

The Syntra Standard Library provides high-performance, memory-safe modules for systems programming, data processing, networking, and concurrency.

---

## 📚 Standard Library Modules

| Module | Namespace | Description |
| :--- | :--- | :--- |
| [**System**](system-and-io.md) | `System` | Standard output, process lifecycle, command-line arguments, environment variables. |
| [**Math**](math-and-file.md) | `Math` | Mathematical functions, IEEE-754 arithmetic, trigonometry, and mathematical constants. |
| [**File**](math-and-file.md) | `File` | Fast synchronous file reading, writing, path existence checks, and deletion. |
| [**Collections & Strings**](collections-and-json.md) | Built-in | Dynamic arrays (`push`, `pop`, `get`, `set`), hash maps, string manipulation. |
| [**Json**](collections-and-json.md) | `Json` | High-speed JSON serialization (`stringify`) and deserialization (`parse`). |
| [**Networking & WebSockets**](networking-and-websockets.md) | `Net` / `Web` | HTTP reactor, HTTP server, RFC 6455 WebSockets, Base64 encoding, SHA-1 crypto. |
| [**Concurrency**](concurrency-primitives.md) | `Task` / `Channel` / `ThreadPool` | Structured concurrent tasks, FIFO channels, and worker thread pools. |
| [**Dependency**](../tools/package-manifest.md) | `Dependency` | Package manifest dependency resolution constructors for `Package.syn`. |
