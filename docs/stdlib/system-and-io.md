# System & I/O (`System`)

The `System` module provides low-level operating system interfaces for console output, process control, environment inspection, and timing.

---

## 📌 Methods

### `System.println(string message): void`
Prints a string to standard output (`stdout`) followed by an OS newline.

```syntra
System.println("Operation succeeded.");
```

### `System.print(string message): void`
Prints a string to standard output (`stdout`) without appending a trailing newline.

```syntra
System.print("Processing [");
System.print("====");
System.println("] 100%");
```

### `System.args(): Array<string>`
Returns an array of command-line arguments passed to the executable.

```syntra
let arguments = System.args();
for arg in arguments {
    System.println("Arg: " + arg);
}
```

### `System.time_now_ms(): int`
Returns the current Unix epoch time in milliseconds.

```syntra
let start = System.time_now_ms();
// ... run heavy computation ...
let elapsed = System.time_now_ms() - start;
System.println("Elapsed: " + elapsed + " ms");
```

### `System.env(string key): string`
Retrieves the value of an environment variable. Returns empty string if not set.

```syntra
let path = System.env("PATH");
let user = System.env("USER");
```

### `System.exit(int code): void`
Terminates the current process immediately with the specified OS exit code.

```syntra
if has_error {
    System.exit(1);
}
```
