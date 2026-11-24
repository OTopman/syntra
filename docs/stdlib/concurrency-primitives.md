# Concurrency Primitives (`Task`, `Channel`, `ThreadPool`)

This chapter provides an exhaustive API reference for the concurrency modules in Syntra.

---

## ⚡ Task Module (`Task`)

### `Task.spawn(fn() action): Task<T>`
Spawns an asynchronous concurrent worker task executing the provided lambda.

```syntra
let task = Task.spawn(fn() {
    let total = 0;
    for i in 0..1000 {
        total = total + i;
    }
    return total;
});
```

### `task.join(): T`
Blocks the calling fiber until the task completes and yields its result.

```syntra
let sum = task.join();
```

### `task.is_done(): bool`
Returns `true` if the task has completed execution.

### `task.cancel(): void`
Signals cancellation to the running task.

### `Task.sleep_ms(int milliseconds): void`
Suspends execution of the current fiber for the specified duration.

```syntra
Task.sleep_ms(250); // Pause for 250 milliseconds
```

---

## 📬 Channel Module (`Channel`)

### `Channel.new(int capacity): Channel<T>`
Instantiates a FIFO message-passing channel. `capacity` determines buffer limit (0 for rendezvous/unbuffered channel).

```syntra
let ch = Channel.new(20);
```

### `ch.send(T message): void`
Sends a message into the channel. If buffer is full, blocks until space becomes available.

```syntra
ch.send("Task payload");
```

### `ch.recv(): T`
Receives and removes the next message from the channel. Blocks if channel is empty.

```syntra
let msg = ch.recv();
```

### `Channel.select(Array<Channel<any>> channels): int`
Multiplexes over an array of channels, returning the integer index of the first channel that is ready for receiving without blocking.

---

## 🏊 ThreadPool Module (`ThreadPool`)

### `new ThreadPool(int workers)`
Initializes a bounded OS thread worker pool with `workers` threads.

```syntra
let pool = new ThreadPool(8);
```

### `pool.submit(fn() job): void`
Enqueues a job into the thread pool's task queue.

### `pool.wait_idle(): void`
Blocks until all queued jobs have finished execution.
