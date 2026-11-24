use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use syntra_syntax::ast::{Block, Expr};

static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);
static NEXT_CHAN_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug)]
pub struct ChannelState {
    pub queue: Mutex<Vec<Value>>,
    pub cond: Condvar,
    pub closed: AtomicBool,
}

impl ChannelState {
    pub fn send(&self, val: Value) {
        let mut q = self.queue.lock().unwrap();
        q.push(val);
        self.cond.notify_one();
    }

    pub fn recv(&self) -> Option<Value> {
        let mut q = self.queue.lock().unwrap();
        loop {
            if !q.is_empty() {
                return Some(q.remove(0));
            }
            if self.closed.load(Ordering::SeqCst) {
                return None;
            }
            q = self.cond.wait(q).unwrap();
        }
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.cond.notify_all();
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }

    pub fn try_recv(&self) -> Option<Value> {
        let mut q = self.queue.lock().unwrap();
        if !q.is_empty() {
            Some(q.remove(0))
        } else {
            None
        }
    }

    pub fn try_send(&self, val: Value) -> bool {
        if self.closed.load(Ordering::SeqCst) {
            return false;
        }
        let mut q = self.queue.lock().unwrap();
        q.push(val);
        self.cond.notify_one();
        true
    }
}

#[derive(Clone)]
pub struct ChannelHandle {
    pub id: usize,
    pub state: Arc<ChannelState>,
}

impl ChannelHandle {
    pub fn new() -> Self {
        Self {
            id: NEXT_CHAN_ID.fetch_add(1, Ordering::Relaxed),
            state: Arc::new(ChannelState {
                queue: Mutex::new(Vec::new()),
                cond: Condvar::new(),
                closed: AtomicBool::new(false),
            }),
        }
    }
}

impl Default for ChannelHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ChannelHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for ChannelHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Channel({})", self.id)
    }
}

#[derive(Clone)]
pub struct TaskHandle {
    pub id: usize,
    pub handle: Arc<Mutex<Option<std::thread::JoinHandle<Value>>>>,
    pub shared_result: Option<Arc<(Mutex<Option<Value>>, Condvar)>>,
}

impl TaskHandle {
    pub fn new(h: std::thread::JoinHandle<Value>) -> Self {
        Self {
            id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
            handle: Arc::new(Mutex::new(Some(h))),
            shared_result: None,
        }
    }

    pub fn new_pool(shared: Arc<(Mutex<Option<Value>>, Condvar)>) -> Self {
        Self {
            id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
            handle: Arc::new(Mutex::new(None)),
            shared_result: Some(shared),
        }
    }

    pub fn await_result(&self) -> Result<Value, String> {
        let mut lock = self.handle.lock().unwrap();
        if let Some(h) = lock.take() {
            match h.join() {
                Ok(val) => Ok(val),
                Err(_) => Err("Task execution panicked".to_string()),
            }
        } else if let Some(ref shared) = self.shared_result {
            let (lock, cvar) = &**shared;
            let mut res_guard = lock.lock().unwrap();
            while res_guard.is_none() {
                res_guard = cvar.wait(res_guard).unwrap();
            }
            Ok(res_guard.clone().unwrap())
        } else {
            Ok(Value::Void)
        }
    }

    pub fn is_done(&self) -> bool {
        let lock = self.handle.lock().unwrap();
        if let Some(ref h) = *lock {
            h.is_finished()
        } else if let Some(ref shared) = self.shared_result {
            let (lock, _) = &**shared;
            lock.lock().unwrap().is_some()
        } else {
            true
        }
    }
}

impl PartialEq for TaskHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for TaskHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Task({})", self.id)
    }
}

static NEXT_POOL_ID: AtomicUsize = AtomicUsize::new(1);

pub type Job = Option<Box<dyn FnOnce() + Send + 'static>>;

#[derive(Clone)]
pub struct ThreadPoolHandle {
    pub id: usize,
    pub workers: usize,
    pub sender: Arc<std::sync::mpsc::Sender<Job>>,
    pub threads: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>,
}

impl ThreadPoolHandle {
    pub fn new(workers: usize) -> Self {
        let workers = if workers == 0 { 4 } else { workers };
        let (tx, rx) = std::sync::mpsc::channel::<Option<Box<dyn FnOnce() + Send + 'static>>>();
        let rx = Arc::new(Mutex::new(rx));
        let mut threads = Vec::with_capacity(workers);

        for _ in 0..workers {
            let rx = Arc::clone(&rx);
            let h = std::thread::spawn(move || {
                while let Ok(msg) = {
                    let lock = rx.lock().unwrap();
                    lock.recv()
                } {
                    match msg {
                        Some(job) => job(),
                        None => break,
                    }
                }
            });
            threads.push(h);
        }

        Self {
            id: NEXT_POOL_ID.fetch_add(1, Ordering::Relaxed),
            workers,
            sender: Arc::new(tx),
            threads: Arc::new(Mutex::new(threads)),
        }
    }
}

impl PartialEq for ThreadPoolHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for ThreadPoolHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ThreadPool({})", self.id)
    }
}

static NEXT_NET_ID: AtomicUsize = AtomicUsize::new(1);
static NEXT_ARENA_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone)]
pub struct ArenaHandle {
    pub id: usize,
    pub capacity: usize,
    pub allocated_bytes: Arc<Mutex<usize>>,
    pub is_destroyed: Arc<Mutex<bool>>,
    pub allocations: Arc<Mutex<Vec<Value>>>,
}

impl ArenaHandle {
    pub fn new(capacity: usize) -> Self {
        Self {
            id: NEXT_ARENA_ID.fetch_add(1, Ordering::Relaxed),
            capacity: if capacity == 0 { 65536 } else { capacity },
            allocated_bytes: Arc::new(Mutex::new(0)),
            is_destroyed: Arc::new(Mutex::new(false)),
            allocations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn alloc(&self, val: Value) -> Result<Value, String> {
        if *self.is_destroyed.lock().unwrap() {
            return Err("Cannot allocate in destroyed Arena domain".to_string());
        }
        let size = match &val {
            Value::Object(_) => 64,
            Value::Array(a) => 32 + a.lock().unwrap().len() * 8,
            Value::String(s) => 24 + s.len(),
            _ => 16,
        };
        *self.allocated_bytes.lock().unwrap() += (size + 7) & !7;
        self.allocations.lock().unwrap().push(val.clone());
        Ok(val)
    }

    pub fn reset(&self) {
        *self.allocated_bytes.lock().unwrap() = 0;
        self.allocations.lock().unwrap().clear();
    }

    pub fn destroy(&self) {
        *self.is_destroyed.lock().unwrap() = true;
        self.reset();
    }

    pub fn allocated_bytes(&self) -> usize {
        *self.allocated_bytes.lock().unwrap()
    }
}

impl PartialEq for ArenaHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for ArenaHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Arena(id={}, allocated={})",
            self.id,
            self.allocated_bytes()
        )
    }
}

#[derive(Clone)]
pub struct TcpListenerHandle {
    pub id: usize,
    pub port: u16,
    pub listener: Arc<Mutex<Option<std::net::TcpListener>>>,
}

impl TcpListenerHandle {
    pub fn new(port: u16, listener: std::net::TcpListener) -> Self {
        Self {
            id: NEXT_NET_ID.fetch_add(1, Ordering::Relaxed),
            port,
            listener: Arc::new(Mutex::new(Some(listener))),
        }
    }
}

impl PartialEq for TcpListenerHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for TcpListenerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TcpListener(port={})", self.port)
    }
}

#[derive(Clone)]
pub struct TcpStreamHandle {
    pub id: usize,
    pub stream: Arc<Mutex<Option<std::net::TcpStream>>>,
}

impl TcpStreamHandle {
    pub fn new(stream: std::net::TcpStream) -> Self {
        Self {
            id: NEXT_NET_ID.fetch_add(1, Ordering::Relaxed),
            stream: Arc::new(Mutex::new(Some(stream))),
        }
    }
}

impl PartialEq for TcpStreamHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for TcpStreamHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TcpStream(id={})", self.id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Arc<Mutex<Vec<Value>>>),
    Map(Arc<Mutex<HashMap<String, Value>>>),
    Object(Arc<Mutex<Instance>>),
    Task(TaskHandle),
    Channel(ChannelHandle),
    ThreadPool(ThreadPoolHandle),
    TcpListener(TcpListenerHandle),
    TcpStream(TcpStreamHandle),
    Arena(ArenaHandle),
    Lambda {
        params: Vec<String>,
        body_expr: Option<Box<Expr>>,
        body_block: Option<Block>,
        captured_scopes: Vec<HashMap<String, Value>>,
    },
    Void,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => {
                if Arc::ptr_eq(a, b) {
                    true
                } else {
                    *a.lock().unwrap() == *b.lock().unwrap()
                }
            }
            (Value::Map(a), Value::Map(b)) => {
                if Arc::ptr_eq(a, b) {
                    true
                } else {
                    *a.lock().unwrap() == *b.lock().unwrap()
                }
            }
            (Value::Object(a), Value::Object(b)) => {
                if Arc::ptr_eq(a, b) {
                    true
                } else {
                    *a.lock().unwrap() == *b.lock().unwrap()
                }
            }
            (Value::Task(a), Value::Task(b)) => a == b,
            (Value::Channel(a), Value::Channel(b)) => a == b,
            (Value::ThreadPool(a), Value::ThreadPool(b)) => a == b,
            (Value::TcpListener(a), Value::TcpListener(b)) => a == b,
            (Value::TcpStream(a), Value::TcpStream(b)) => a == b,
            (Value::Arena(a), Value::Arena(b)) => a == b,
            (Value::Void, Value::Void) => true,
            _ => false,
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.lock().unwrap().is_empty(),
            Value::Map(m) => !m.lock().unwrap().is_empty(),
            Value::Object(_) => true,
            Value::Task(_) => true,
            Value::Channel(_) => true,
            Value::ThreadPool(_) => true,
            Value::TcpListener(_) => true,
            Value::TcpStream(_) => true,
            Value::Arena(_) => true,
            Value::Lambda { .. } => true,
            Value::Void => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(fl) => write!(f, "{fl}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Array(arr) => {
                let elems: Vec<String> =
                    arr.lock().unwrap().iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", elems.join(", "))
            }
            Value::Map(m) => {
                let mut entries: Vec<String> = m
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| format!("\"{k}\": {v}"))
                    .collect();
                entries.sort();
                write!(f, "{{{}}}", entries.join(", "))
            }
            Value::Object(obj) => {
                let inst = obj.lock().unwrap();
                write!(f, "<{} instance>", inst.class_name)
            }
            Value::Task(t) => write!(f, "<Task #{}>", t.id),
            Value::Channel(c) => write!(f, "<Channel #{}>", c.id),
            Value::ThreadPool(p) => write!(f, "<ThreadPool #{} ({} workers)>", p.id, p.workers),
            Value::TcpListener(l) => write!(f, "<TcpListener port={}>", l.port),
            Value::TcpStream(s) => write!(f, "<TcpStream #{}>", s.id),
            Value::Arena(a) => write!(
                f,
                "<Arena #{} (allocated={} bytes)>",
                a.id,
                a.allocated_bytes()
            ),
            Value::Lambda { params, .. } => {
                write!(f, "<lambda({})>", params.join(", "))
            }
            Value::Void => write!(f, "void"),
        }
    }
}
