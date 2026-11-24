# Structured Concurrency

Syntra provides first-class concurrency primitives built directly into the runtime without callback hell or unbounded thread spawning.

---

## 🧵 Core Primitives

Syntra concurrency centers on three main constructs:

| Construct | Description |
| :--- | :--- |
| `Task<T>` | Lightweight asynchronous concurrent execution fiber. |
| `Channel<T>` | Thread-safe, bounded or unbounded FIFO message-passing channel. |
| `ThreadPool` | Bounded multi-worker execution pool with dynamic load balancing. |

---

## 🚀 Tasks (`Task.spawn`)

Spawn an asynchronous concurrent task using `Task.spawn`:

```syntra
func compute_heavy_task(int value): int {
    System.println("Computing for " + value);
    return value * value;
}

func main(): void {
    let task = Task.spawn(fn() {
        return compute_heavy_task(42);
    });

    // Main execution continues concurrently...
    System.println("Task spawned. Waiting for result...");

    let result = task.join();
    System.println("Computed result: " + result);
}
```

---

## 📬 Channels (`Channel.new`, `send`, `recv`)

Channels allow thread-safe message passing between concurrent tasks without shared memory locks:

```syntra
func producer(Channel<string> ch): void {
    ch.send("Packet 1");
    ch.send("Packet 2");
    ch.send("Packet 3");
}

func consumer(Channel<string> ch): void {
    for i in 0..3 {
        let msg = ch.recv();
        System.println("Consumed: " + msg);
    }
}

func main(): void {
    // Create bounded FIFO channel with buffer capacity of 10
    let ch = Channel.new(10);

    Task.spawn(fn() { producer(ch); });
    Task.spawn(fn() { consumer(ch); });
}
```

---

## 🔀 Channel Select (`Channel.select`)

Non-blocking multiplexer over multiple channel operations:

```syntra
func main(): void {
    let ch1 = Channel.new(5);
    let ch2 = Channel.new(5);

    ch2.send("urgent alert");

    // Select returns the index of the first channel ready for reading
    let readyIndex = Channel.select([ch1, ch2]);
    if readyIndex == 1 {
        let alert = ch2.recv();
        System.println("Received from ch2: " + alert);
    }
}
```

---

## 🏊 ThreadPool (`ThreadPool.new`)

For CPU-intensive worker batches, instantiate a bounded thread pool:

```syntra
func main(): void {
    let pool = new ThreadPool(4); // 4 dedicated worker threads

    for i in 0..8 {
        pool.submit(fn() {
            System.println("Executing job #" + i + " on threadpool");
        });
    }

    pool.wait_idle();
}
```
