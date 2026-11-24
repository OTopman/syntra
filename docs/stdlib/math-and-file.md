# Math & File Operations

This chapter covers the `Math` and `File` standard library modules.

---

## 🧮 Math Module (`Math`)

The `Math` module provides IEEE-754 floating-point operations and constants.

### Constants
* `Math.PI`: Value of $\pi \approx 3.141592653589793$
* `Math.E`: Euler's number $e \approx 2.718281828459045$

### Functions
* `Math.sqrt(float x): float`: Computes the non-negative square root of $x$.
* `Math.pow(float base, float exp): float`: Computes $\text{base}^{\text{exp}}$.
* `Math.abs(float x): float`: Returns the absolute value of $x$.
* `Math.min(number a, number b): number`: Returns the smaller of two numbers.
* `Math.max(number a, number b): number`: Returns the greater of two numbers.
* `Math.floor(float x): int`: Rounds down to the nearest integer.
* `Math.ceil(float x): int`: Rounds up to the nearest integer.
* `Math.sin(float radians): float`: Computes the trigonometric sine.
* `Math.cos(float radians): float`: Computes the trigonometric cosine.
* `Math.tan(float radians): float`: Computes the trigonometric tangent.

```syntra
let hyp = Math.sqrt(Math.pow(3.0, 2.0) + Math.pow(4.0, 2.0));
System.println("Hypotenuse: " + hyp); // 5.0
```

---

## 📁 File Module (`File`)

The `File` module provides synchronous filesystem operations.

### `File.read_to_string(string path): string`
Reads an entire file into a UTF-8 encoded string. Throws an exception if the file cannot be opened.

```syntra
let content = File.read_to_string("data/input.txt");
```

### `File.write_string(string path, string content): void`
Writes string content to the target file path, creating or truncating it.

```syntra
File.write_string("output.log", "Build finished cleanly.\n");
```

### `File.exists(string path): bool`
Returns `true` if the target path exists on the filesystem.

```syntra
if File.exists("config.json") {
    let conf = File.read_to_string("config.json");
}
```

### `File.delete(string path): void`
Deletes the file at the specified path.

```syntra
File.delete("temp_cache.bin");
```
