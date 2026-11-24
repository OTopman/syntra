# Lexical Structure

This chapter describes the lexical tokens, grammar, comments, and identifiers in Syntra.

---

## 🔤 Character Set & Encoding

Syntra source files are UTF-8 encoded plain text files conventionally using the `.syn` (or `.sy`) file extension.

---

## 💬 Comments

Syntra supports three forms of comments:

```syntra
// Single-line comment: continues to the end of the line.

/* Multi-line block comment:
   can span across multiple lines. */

/// Documentation comment: parsed by `syn doc` and exposed in editor hover documentation.
func compute_total(int amount): int {
    return amount;
}
```

---

## 🔑 Reserved Keywords

The following tokens are reserved keywords in Syntra:

| Category | Keywords |
| :--- | :--- |
| **Control Flow** | `if`, `else`, `while`, `for`, `in`, `return`, `try`, `catch`, `finally`, `throw`, `select`, `case`, `default` |
| **Declarations** | `func`, `fn`, `let`, `const`, `mut`, `class`, `interface`, `namespace`, `package`, `import` |
| **Visibility & Modifiers** | `public`, `private`, `static`, `override`, `implements`, `extends` |
| **Concurrency** | `async`, `await` |
| **Types** | `int`, `float`, `bool`, `string`, `void` |
| **Literals** | `true`, `false`, `null`, `this`, `super`, `new` |

---

## 🔢 Literals

### Integers
Integers represent 64-bit signed two's complement integers (`-9223372036854775808` to `9223372036854775807`).

```syntra
let decimal = 42;
let hex = 0x2A;
let binary = 0b101010;
let with_underscores = 1_000_000;
```

### Floating-Point Numbers
Floats represent 64-bit IEEE-754 double-precision numbers.

```syntra
let pi = 3.141592653589793;
let small = 0.005;
let scientific = 1.5e-3;
```

### Strings & String Interpolation
Strings are immutable, UTF-8 encoded byte sequences enclosed in double quotes (`"`).

Escape sequences supported:
* `\n`: Newline
* `\r`: Carriage return
* `\t`: Tab
* `\\`: Backslash
* `\"`: Double quote
* `\xHH`: Hexadecimal byte
* `\u{...}`: Unicode code point

String interpolation uses `\{expression}`:

```syntra
let name = "Alice";
let count = 5;
let greeting = "Hello, \{name}! You have \{count * 2} items.";
```

---

## ➕ Operators

| Category | Operators |
| :--- | :--- |
| **Arithmetic** | `+`, `-`, `*`, `/`, `%` |
| **Comparison** | `==`, `!=`, `<`, `<=`, `>`, `>=` |
| **Logical** | `&&`, `\|\|`, `!` |
| **Bitwise** | `&`, `\|`, `^`, `~`, `<<`, `>>` |
| **Assignment** | `=`, `+=`, `-=`, `*=`, `/=` |
| **Range** | `..` (e.g. `0..10`) |
| **Arrow** | `=>`, `->` |
