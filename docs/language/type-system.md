# Type System

Syntra features a static, nominal, and structural type system designed for strict memory safety, zero-cost abstraction, and runtime predictability.

---

## 🧱 Primitive Types

| Type | Description | Memory Size | Example |
| :--- | :--- | :--- | :--- |
| `int` | 64-bit signed two's complement integer | 8 bytes | `let count: int = 100;` |
| `float` | 64-bit IEEE-754 double precision float | 8 bytes | `let ratio: float = 0.75;` |
| `bool` | Boolean truth value (`true` or `false`) | 1 byte | `let ready: bool = true;` |
| `string` | Immutable UTF-8 text string | 16 bytes (ptr + len) | `let label: string = "Syntra";` |
| `void` | Unit type representing absence of value | 0 bytes | `func log(): void {}` |

---

## 📦 Generic Composite Types

### Dynamic Arrays (`Array<T>` or `T[]`)
Arrays in Syntra are dynamically sized contiguous buffer collections:

```syntra
let numbers: int[] = [1, 2, 3, 4, 5];
numbers.push(6);
let first = numbers.get(0);
let count = numbers.length();
```

### Hash Maps (`Map<K, V>`)
Generic key-value hash map storage:

```syntra
let scores = new Map<string, int>();
scores.set("alex", 95);
scores.set("bob", 88);

if scores.has("alex") {
    let s = scores.get("alex");
    System.println("Alex score: " + s);
}
```

---

## 🏷️ Variable Declarations (`let` and `const`)

### Local Variables (`let`)
Variables declared with `let` are statically typed with automatic type inference:

```syntra
let x = 10;                // Inferred as int
let name = "Syntra";       // Inferred as string
let ratio: float = 3.14;   // Explicit type annotation
```

### Compile-Time Constants (`const`)
Constants are immutable values known at compile time:

```syntra
const int MAX_BUFFER_SIZE = 4096;
const string DEFAULT_HOST = "127.0.0.1";
```

---

## 🛡️ Null Safety

In Syntra, primitive types (`int`, `float`, `bool`) cannot be null. Reference types default to non-null unless wrapped in an explicit container or checked conditionally:

```syntra
let user = new User(1, "alice");
if user != null {
    user.display();
}
```
