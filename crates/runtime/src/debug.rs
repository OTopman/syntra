use crate::value::Value;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepMode {
    Continue,
    StepInto,
    StepOver(usize), // Target stack depth
    StepOut(usize),  // Target stack depth
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseReason {
    Entry,
    Breakpoint { file: String, line: usize },
    Step,
    Exception(String),
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub id: usize,
    pub fn_name: String,
    pub file: String,
    pub line: usize,
    pub variables: HashMap<String, Value>,
}

#[derive(Debug)]
pub enum DebugCommand {
    Continue,
    StepInto,
    StepOver,
    StepOut,
    SetBreakpoint {
        file: String,
        line: usize,
    },
    RemoveBreakpoint {
        file: String,
        line: usize,
    },
    GetStackTrace(Sender<Vec<StackFrame>>),
    GetVariables {
        frame_id: usize,
        reply: Sender<HashMap<String, Value>>,
    },
    Evaluate {
        expr: String,
        frame_id: usize,
        reply: Sender<Result<Value, String>>,
    },
    Quit,
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    Paused {
        reason: PauseReason,
        file: String,
        line: usize,
        fn_name: String,
    },
    Output(String),
    Terminated {
        exit_code: i64,
    },
}

pub struct DebugHookState {
    pub breakpoints: HashMap<String, HashSet<usize>>,
    pub step_mode: StepMode,
    pub cmd_rx: Receiver<DebugCommand>,
    pub event_tx: Sender<DebugEvent>,
    pub call_stack: Vec<(String, String, usize)>, // (fn_name, file, line)
}

pub struct DebugHandle {
    pub cmd_tx: Sender<DebugCommand>,
    pub event_rx: Receiver<DebugEvent>,
}

impl DebugHandle {
    pub fn send(&self, cmd: DebugCommand) -> Result<(), mpsc::SendError<DebugCommand>> {
        self.cmd_tx.send(cmd)
    }

    pub fn recv_event(&self) -> Result<DebugEvent, mpsc::RecvError> {
        self.event_rx.recv()
    }

    pub fn try_recv_event(&self) -> Result<DebugEvent, mpsc::TryRecvError> {
        self.event_rx.try_recv()
    }
}

pub fn create_debug_channel() -> (Arc<Mutex<DebugHookState>>, DebugHandle) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(DebugHookState {
        breakpoints: HashMap::new(),
        step_mode: StepMode::StepInto,
        cmd_rx,
        event_tx,
        call_stack: Vec::new(),
    }));
    let handle = DebugHandle { cmd_tx, event_rx };
    (state, handle)
}
