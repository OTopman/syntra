# Functions & Closures

Functions are first-class citizens in Syntra. They can be declared globally, associated with namespaces, defined as class methods, or created anonymously as closures.

---

## 🛠️ Function Declarations

Named functions are declared using either `func` or `fn`:

```syntra
func add(int a, int b): int {
    return a + b;
}

func greet(string name): void {
    System.println("Hello, " + name + "!");
}
```

### Parameters & Return Types
Every parameter must have a declared type. Return types are declared following a colon `:`. If no return value is produced, declare `: void`.

---

## 🎯 Anonymous Lambda Closures (`fn`)

Syntra closures are defined using `fn(params) { body }`. Closures automatically capture variables from their lexical enclosing scope:

```syntra
func main(): void {
    let multiplier = 3;

    // Anonymous lambda capturing 'multiplier'
    let scale = fn(int x) {
        return x * multiplier;
    };

    let result = scale(7);
    System.println("Scaled: " + result); // Output: Scaled: 21
}
```

---

## 🔄 Higher-Order Functions

Functions can accept closures as arguments or return them:

```syntra
func apply_transformation(int[] data, fn(int): int transform): int[] {
    let output: int[] = [];
    for item in data {
        output.push(transform(item));
    }
    return output;
}

func main(): void {
    let raw = [1, 2, 3, 4];
    let squared = apply_transformation(raw, fn(int n) {
        return n * n;
    });
    // squared is [1, 4, 9, 16]
}
```
