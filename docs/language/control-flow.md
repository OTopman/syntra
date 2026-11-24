# Control Flow

Syntra provides structured control flow constructs for branching, iterative looping, and exception management.

---

## 🔀 Conditionals (`if` / `else`)

Parentheses around the condition are optional; curly braces `{}` are mandatory.

```syntra
let score = 85;

if score >= 90 {
    System.println("Grade: A");
} else if score >= 80 {
    System.println("Grade: B");
} else {
    System.println("Grade: C");
}
```

---

## 🔁 Loops

### While Loop
Executes as long as the condition evaluates to `true`:

```syntra
let counter = 0;
while counter < 5 {
    System.println("Count: " + counter);
    counter = counter + 1;
}
```

### For-In Collection Loop
Iterates over each element in an array or collection:

```syntra
let fruits = ["apple", "banana", "cherry"];
for fruit in fruits {
    System.println("Fruit: " + fruit);
}
```

### For-In Range Loop
Iterates through numerical integer ranges using `start..end`:

```syntra
// Iterates from 0 up to (exclusive) 10: 0, 1, ..., 9
for i in 0..10 {
    System.println("Index: " + i);
}
```

---

## ⚠️ Exception Handling (`try` / `catch` / `finally` / `throw`)

Syntra provides safe exception unwinding:

```syntra
func read_config_file(string path): string {
    if !File.exists(path) {
        throw "Configuration file not found: " + path;
    }
    return File.read_to_string(path);
}

func main(): void {
    try {
        let content = read_config_file("settings.json");
        System.println("Loaded config: " + content);
    } catch (err) {
        System.println("Caught error: " + err);
    } finally {
        System.println("Cleaned up resources.");
    }
}
```
