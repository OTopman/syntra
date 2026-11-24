# Collections & JSON

This chapter details dynamic arrays, hash maps, string member methods, and the `Json` module.

---

## 📚 Dynamic Arrays (`Array<T>`)

Arrays are dynamically resizable buffers with indexed access:

* `arr.length(): int`: Returns the total number of elements.
* `arr.push(item): void`: Appends an item to the end of the array.
* `arr.pop(): T`: Removes and returns the last element.
* `arr.get(int index): T`: Retrieves element at index (zero-based).
* `arr.set(int index, T value): void`: Replaces element at index.
* `arr.contains(item): bool`: Checks if item exists in the array.

```syntra
let list = [10, 20, 30];
list.push(40);
System.println("Length: " + list.length()); // 4
System.println("Element 2: " + list.get(2)); // 30
```

---

## 🗺️ Hash Maps (`Map<K, V>`)

Associative hash maps mapping keys to values:

* `map.set(K key, V value): void`: Inserts or updates key-value mapping.
* `map.get(K key): V`: Retrieves value for key.
* `map.has(K key): bool`: Returns `true` if key exists in the map.
* `map.keys(): Array<K>`: Returns array containing all keys.
* `map.values(): Array<V>`: Returns array containing all values.

```syntra
let cache = new Map<string, string>();
cache.set("session_1", "authenticated");

if cache.has("session_1") {
    let status = cache.get("session_1");
    System.println("Status: " + status);
}
```

---

## 🔡 String Operations

* `str.length(): int`: Returns string character count.
* `str.substring(int start, int end): string`: Extracts slice from start to end index.
* `str.to_upper(): string`: Converts string to uppercase.
* `str.to_lower(): string`: Converts string to lowercase.
* `str.trim(): string`: Trims leading and trailing whitespace.
* `str.split(string delimiter): Array<string>`: Splits string by delimiter.

```syntra
let text = "  syntra programming language  ";
let clean = text.trim().to_upper();
System.println(clean); // "SYNTRA PROGRAMMING LANGUAGE"

let tokens = clean.split(" ");
// tokens is ["SYNTRA", "PROGRAMMING", "LANGUAGE"]
```

---

## 📄 JSON Module (`Json`)

### `Json.stringify(object): string`
Serializes any Syntra instance, array, or map into a formatted JSON string:

```syntra
let payload = new Map<string, any>();
payload.set("service", "auth");
payload.set("port", 8080);
payload.set("active", true);

let json_str = Json.stringify(payload);
System.println("Serialized: " + json_str);
```

### `Json.parse(string json_str): any`
Parses a JSON string into Syntra Map and Array structures:

```syntra
let raw = "{\"id\": 42, \"name\": \"Syntra\"}";
let parsed = Json.parse(raw);
System.println("ID: " + parsed.get("id"));
```
