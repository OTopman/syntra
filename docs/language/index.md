# Language Reference Manual

Welcome to the comprehensive Syntra Language Reference (Edition 2026).

Syntra combines modern syntax with static type safety, explicit memory domain boundaries, structured concurrency, and empirical self-optimization.

---

## 📑 Language Manual Chapters

1. [**Lexical Structure**](lexical-structure.md)
   * Keywords, identifiers, numeric literals, strings, interpolation, operators, and comments.

2. [**Type System**](type-system.md)
   * Primitive types (`int`, `float`, `bool`, `string`, `void`), arrays, maps, type inference with `let`, and compile-time constants (`const`).

3. [**Control Flow**](control-flow.md)
   * Conditional statements (`if`/`else`), while loops, for-in collection and range loops, and exception handling (`try`/`catch`/`finally`).

4. [**Functions & Closures**](functions-and-closures.md)
   * Named functions (`func` and `fn`), return types, anonymous lambda closures, lexical environment capture, and recursion.

5. [**Object-Oriented Programming**](object-oriented.md)
   * Classes, fields, constructors (`new`), member access (`this`), visibility modifiers (`public`/`private`), interfaces, and polymorphism.

6. [**Structured Concurrency**](async-and-concurrency.md)
   * Asynchronous tasks (`Task<T>`), worker pools (`ThreadPool`), FIFO channels (`Channel<T>`), and non-blocking multiplexing with `Channel.select()`.

7. [**Memory Domains & Safety**](memory-domains.md)
   * Linear ownership, strict memory safety, zero-leak domain isolation (MEM-001..004), and arena allocators.
