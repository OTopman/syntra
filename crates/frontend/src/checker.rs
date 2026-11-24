use crate::symbols::*;
use crate::types::Type;
use std::collections::HashMap;
use syntra_diagnostics::{Diagnostic, Span};
use syntra_syntax::ast::*;

pub struct TypeChecker {
    symbols: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    current_return_type: Option<Type>,
    current_class: Option<String>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
            current_return_type: None,
            current_class: None,
        };
        checker.register_builtins();
        checker
    }

    fn register_builtins(&mut self) {
        let mut file_fields = HashMap::new();
        file_fields.insert(
            "path".to_string(),
            FieldSymbol {
                name: "path".to_string(),
                ty: Type::String,
                is_public: true,
                is_static: false,
                span: Span::default(),
            },
        );

        let mut file_methods = HashMap::new();
        file_methods.insert(
            "exists".to_string(),
            MethodSymbol {
                name: "exists".to_string(),
                params: vec![],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        file_methods.insert(
            "readText".to_string(),
            MethodSymbol {
                name: "readText".to_string(),
                params: vec![],
                return_type: Type::String,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        file_methods.insert(
            "writeText".to_string(),
            MethodSymbol {
                name: "writeText".to_string(),
                params: vec![("content".to_string(), Type::String)],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        file_methods.insert(
            "appendText".to_string(),
            MethodSymbol {
                name: "appendText".to_string(),
                params: vec![("content".to_string(), Type::String)],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        file_methods.insert(
            "delete".to_string(),
            MethodSymbol {
                name: "delete".to_string(),
                params: vec![],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        file_methods.insert(
            "size".to_string(),
            MethodSymbol {
                name: "size".to_string(),
                params: vec![],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        file_methods.insert(
            "getPath".to_string(),
            MethodSymbol {
                name: "getPath".to_string(),
                params: vec![],
                return_type: Type::String,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        file_methods.insert(
            "readLines".to_string(),
            MethodSymbol {
                name: "readLines".to_string(),
                params: vec![],
                return_type: Type::Array(Box::new(Type::String)),
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );

        self.symbols.classes.insert(
            "File".to_string(),
            ClassSymbol {
                name: "File".to_string(),
                generic_params: Vec::new(),
                extends: None,
                implements: vec![],
                fields: file_fields,
                methods: file_methods,
                constructor: Some(ConstructorSymbol {
                    params: vec![("path".to_string(), Type::String)],
                    span: Span::default(),
                }),
                span: Span::default(),
            },
        );

        let mut map_methods = HashMap::new();
        map_methods.insert(
            "set".to_string(),
            MethodSymbol {
                name: "set".to_string(),
                params: vec![
                    ("key".to_string(), Type::String),
                    ("val".to_string(), Type::Unknown),
                ],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "get".to_string(),
            MethodSymbol {
                name: "get".to_string(),
                params: vec![("key".to_string(), Type::String)],
                return_type: Type::Unknown,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "has".to_string(),
            MethodSymbol {
                name: "has".to_string(),
                params: vec![("key".to_string(), Type::String)],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "remove".to_string(),
            MethodSymbol {
                name: "remove".to_string(),
                params: vec![("key".to_string(), Type::String)],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "delete".to_string(),
            MethodSymbol {
                name: "delete".to_string(),
                params: vec![("key".to_string(), Type::String)],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "len".to_string(),
            MethodSymbol {
                name: "len".to_string(),
                params: vec![],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "size".to_string(),
            MethodSymbol {
                name: "size".to_string(),
                params: vec![],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "keys".to_string(),
            MethodSymbol {
                name: "keys".to_string(),
                params: vec![],
                return_type: Type::Array(Box::new(Type::String)),
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "values".to_string(),
            MethodSymbol {
                name: "values".to_string(),
                params: vec![],
                return_type: Type::Array(Box::new(Type::Unknown)),
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        map_methods.insert(
            "clear".to_string(),
            MethodSymbol {
                name: "clear".to_string(),
                params: vec![],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );

        self.symbols.classes.insert(
            "Map".to_string(),
            ClassSymbol {
                name: "Map".to_string(),
                generic_params: vec!["K".to_string(), "V".to_string()],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                methods: map_methods,
                constructor: Some(ConstructorSymbol {
                    params: vec![],
                    span: Span::default(),
                }),
                span: Span::default(),
            },
        );

        // Built-in Task class
        let mut task_methods = HashMap::new();
        task_methods.insert(
            "spawn".to_string(),
            MethodSymbol {
                name: "spawn".to_string(),
                params: vec![("func".to_string(), Type::Unknown)],
                return_type: Type::Class("Task".to_string()),
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );
        task_methods.insert(
            "sleep".to_string(),
            MethodSymbol {
                name: "sleep".to_string(),
                params: vec![("ms".to_string(), Type::Int)],
                return_type: Type::Void,
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );
        task_methods.insert(
            "join".to_string(),
            MethodSymbol {
                name: "join".to_string(),
                params: vec![],
                return_type: Type::Unknown,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        task_methods.insert(
            "wait".to_string(),
            MethodSymbol {
                name: "wait".to_string(),
                params: vec![],
                return_type: Type::Unknown,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        task_methods.insert(
            "await".to_string(),
            MethodSymbol {
                name: "await".to_string(),
                params: vec![],
                return_type: Type::Unknown,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        task_methods.insert(
            "isDone".to_string(),
            MethodSymbol {
                name: "isDone".to_string(),
                params: vec![],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        task_methods.insert(
            "isFinished".to_string(),
            MethodSymbol {
                name: "isFinished".to_string(),
                params: vec![],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );

        self.symbols.classes.insert(
            "Task".to_string(),
            ClassSymbol {
                name: "Task".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                methods: task_methods,
                constructor: None,
                span: Span::default(),
            },
        );

        // Built-in Channel class
        let mut chan_methods = HashMap::new();
        chan_methods.insert(
            "send".to_string(),
            MethodSymbol {
                name: "send".to_string(),
                params: vec![("val".to_string(), Type::GenericParam("T".to_string()))],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "recv".to_string(),
            MethodSymbol {
                name: "recv".to_string(),
                params: vec![],
                return_type: Type::GenericParam("T".to_string()),
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "receive".to_string(),
            MethodSymbol {
                name: "receive".to_string(),
                params: vec![],
                return_type: Type::GenericParam("T".to_string()),
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "close".to_string(),
            MethodSymbol {
                name: "close".to_string(),
                params: vec![],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "len".to_string(),
            MethodSymbol {
                name: "len".to_string(),
                params: vec![],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "size".to_string(),
            MethodSymbol {
                name: "size".to_string(),
                params: vec![],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "isEmpty".to_string(),
            MethodSymbol {
                name: "isEmpty".to_string(),
                params: vec![],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "tryRecv".to_string(),
            MethodSymbol {
                name: "tryRecv".to_string(),
                params: vec![],
                return_type: Type::GenericParam("T".to_string()),
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "trySend".to_string(),
            MethodSymbol {
                name: "trySend".to_string(),
                params: vec![("val".to_string(), Type::GenericParam("T".to_string()))],
                return_type: Type::Bool,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        chan_methods.insert(
            "select".to_string(),
            MethodSymbol {
                name: "select".to_string(),
                params: vec![(
                    "channels".to_string(),
                    Type::Array(Box::new(Type::Class("Channel".to_string()))),
                )],
                return_type: Type::Int,
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );

        self.symbols.classes.insert(
            "Channel".to_string(),
            ClassSymbol {
                name: "Channel".to_string(),
                generic_params: vec!["T".to_string()],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                methods: chan_methods,
                constructor: Some(ConstructorSymbol {
                    params: vec![],
                    span: Span::default(),
                }),
                span: Span::default(),
            },
        );

        // Built-in ThreadPool class
        let mut tp_methods = HashMap::new();
        tp_methods.insert(
            "submit".to_string(),
            MethodSymbol {
                name: "submit".to_string(),
                params: vec![
                    ("fn".to_string(), Type::Unknown),
                    ("arg".to_string(), Type::Unknown),
                ],
                return_type: Type::Class("Task".to_string()),
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        tp_methods.insert(
            "shutdown".to_string(),
            MethodSymbol {
                name: "shutdown".to_string(),
                params: vec![],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        tp_methods.insert(
            "workers".to_string(),
            MethodSymbol {
                name: "workers".to_string(),
                params: vec![],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );

        self.symbols.classes.insert(
            "ThreadPool".to_string(),
            ClassSymbol {
                name: "ThreadPool".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                methods: tp_methods,
                constructor: Some(ConstructorSymbol {
                    params: vec![("workers".to_string(), Type::Int)],
                    span: Span::default(),
                }),
                span: Span::default(),
            },
        );

        // Built-in TcpListener class
        let mut listener_methods = HashMap::new();
        listener_methods.insert(
            "serveReactor".to_string(),
            MethodSymbol {
                name: "serveReactor".to_string(),
                params: vec![
                    ("port".to_string(), Type::Int),
                    ("workers".to_string(), Type::Int),
                ],
                return_type: Type::Int,
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );
        listener_methods.insert(
            "listen".to_string(),
            MethodSymbol {
                name: "listen".to_string(),
                params: vec![("port".to_string(), Type::Int)],
                return_type: Type::Class("TcpListener".to_string()),
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );
        listener_methods.insert(
            "accept".to_string(),
            MethodSymbol {
                name: "accept".to_string(),
                params: vec![],
                return_type: Type::Class("TcpStream".to_string()),
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        listener_methods.insert(
            "port".to_string(),
            MethodSymbol {
                name: "port".to_string(),
                params: vec![],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        listener_methods.insert(
            "close".to_string(),
            MethodSymbol {
                name: "close".to_string(),
                params: vec![],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        self.symbols.classes.insert(
            "TcpListener".to_string(),
            ClassSymbol {
                name: "TcpListener".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                methods: listener_methods,
                constructor: None,
                span: Span::default(),
            },
        );

        // Built-in TcpStream class
        let mut stream_methods = HashMap::new();
        stream_methods.insert(
            "connect".to_string(),
            MethodSymbol {
                name: "connect".to_string(),
                params: vec![
                    ("host".to_string(), Type::String),
                    ("port".to_string(), Type::Int),
                ],
                return_type: Type::Class("TcpStream".to_string()),
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );
        stream_methods.insert(
            "computeWsAccept".to_string(),
            MethodSymbol {
                name: "computeWsAccept".to_string(),
                params: vec![("key".to_string(), Type::String)],
                return_type: Type::String,
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );
        stream_methods.insert(
            "read".to_string(),
            MethodSymbol {
                name: "read".to_string(),
                params: vec![],
                return_type: Type::String,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        stream_methods.insert(
            "readWsFrame".to_string(),
            MethodSymbol {
                name: "readWsFrame".to_string(),
                params: vec![],
                return_type: Type::String,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        stream_methods.insert(
            "write".to_string(),
            MethodSymbol {
                name: "write".to_string(),
                params: vec![("data".to_string(), Type::String)],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        stream_methods.insert(
            "writeWsFrame".to_string(),
            MethodSymbol {
                name: "writeWsFrame".to_string(),
                params: vec![("data".to_string(), Type::String)],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        stream_methods.insert(
            "writeWsClientFrame".to_string(),
            MethodSymbol {
                name: "writeWsClientFrame".to_string(),
                params: vec![("data".to_string(), Type::String)],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        stream_methods.insert(
            "close".to_string(),
            MethodSymbol {
                name: "close".to_string(),
                params: vec![],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        self.symbols.classes.insert(
            "TcpStream".to_string(),
            ClassSymbol {
                name: "TcpStream".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                methods: stream_methods,
                constructor: None,
                span: Span::default(),
            },
        );

        // Built-in Arena class (Memory Domain - Specification 02)
        let mut arena_methods = HashMap::new();
        arena_methods.insert(
            "create".to_string(),
            MethodSymbol {
                name: "create".to_string(),
                params: vec![],
                return_type: Type::Class("Arena".to_string()),
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );
        arena_methods.insert(
            "scoped".to_string(),
            MethodSymbol {
                name: "scoped".to_string(),
                params: vec![("action".to_string(), Type::Unknown)],
                return_type: Type::Unknown,
                is_public: true,
                is_static: true,
                is_override: false,
                span: Span::default(),
            },
        );
        arena_methods.insert(
            "alloc".to_string(),
            MethodSymbol {
                name: "alloc".to_string(),
                params: vec![("obj".to_string(), Type::Unknown)],
                return_type: Type::Unknown,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        arena_methods.insert(
            "reset".to_string(),
            MethodSymbol {
                name: "reset".to_string(),
                params: vec![],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        arena_methods.insert(
            "destroy".to_string(),
            MethodSymbol {
                name: "destroy".to_string(),
                params: vec![],
                return_type: Type::Void,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );
        arena_methods.insert(
            "allocatedBytes".to_string(),
            MethodSymbol {
                name: "allocatedBytes".to_string(),
                params: vec![],
                return_type: Type::Int,
                is_public: true,
                is_static: false,
                is_override: false,
                span: Span::default(),
            },
        );

        self.symbols.classes.insert(
            "Arena".to_string(),
            ClassSymbol {
                name: "Arena".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                methods: arena_methods,
                constructor: Some(ConstructorSymbol {
                    params: vec![],
                    span: Span::default(),
                }),
                span: Span::default(),
            },
        );
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), Vec<Diagnostic>> {
        // Pass 1: Collect all top-level symbols (functions, classes, interfaces)
        for item in &program.items {
            match item {
                Item::Interface(iface) => self.register_interface(iface),
                Item::Class(cls) => self.register_class(cls),
                Item::Function(func) => self.register_function(func),
                Item::Namespace(ns) => self.register_namespace(ns),
                Item::Import(_) => {}
            }
        }

        // Pass 2: Check interface conformance for all classes
        for item in &program.items {
            match item {
                Item::Class(cls) => self.check_interface_conformance(cls),
                Item::Namespace(ns) => {
                    for sub in &ns.items {
                        if let Item::Class(cls) = sub {
                            self.check_interface_conformance(cls);
                        }
                    }
                }
                _ => {}
            }
        }

        // Pass 3: Type check all bodies (functions, methods, constructors)
        for item in &program.items {
            match item {
                Item::Function(func) => self.check_function(func),
                Item::Class(cls) => self.check_class_body(cls),
                Item::Namespace(ns) => self.check_namespace_body(ns),
                Item::Interface(_) | Item::Import(_) => {}
            }
        }

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(self.diagnostics.clone())
        }
    }

    fn register_namespace(&mut self, ns: &NamespaceDecl) {
        for item in &ns.items {
            match item {
                Item::Function(func) => {
                    let mut namespaced_func = func.clone();
                    namespaced_func.name = format!("{}_{}", ns.name, func.name);
                    self.register_function(&namespaced_func);
                }
                Item::Class(cls) => {
                    let mut namespaced_cls = cls.clone();
                    namespaced_cls.name = format!("{}_{}", ns.name, cls.name);
                    self.register_class(&namespaced_cls);
                }
                Item::Interface(iface) => {
                    let mut namespaced_iface = iface.clone();
                    namespaced_iface.name = format!("{}_{}", ns.name, iface.name);
                    self.register_interface(&namespaced_iface);
                }
                _ => {}
            }
        }
    }

    fn check_namespace_body(&mut self, ns: &NamespaceDecl) {
        for item in &ns.items {
            match item {
                Item::Function(func) => {
                    let mut namespaced_func = func.clone();
                    namespaced_func.name = format!("{}_{}", ns.name, func.name);
                    self.check_function(&namespaced_func);
                }
                Item::Class(cls) => {
                    let mut namespaced_cls = cls.clone();
                    namespaced_cls.name = format!("{}_{}", ns.name, cls.name);
                    self.check_class_body(&namespaced_cls);
                }
                _ => {}
            }
        }
    }

    pub fn into_symbols(self) -> SymbolTable {
        self.symbols
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error("LANG-020", message, span));
    }

    // --- Pass 1: Registration ---
    fn register_interface(&mut self, iface: &InterfaceDecl) {
        let mut methods = HashMap::new();
        for sig in &iface.methods {
            let params = sig
                .params
                .iter()
                .map(|p| (p.name.clone(), Type::from_ast(&p.ty)))
                .collect();
            let return_type = sig
                .return_type
                .as_ref()
                .map(Type::from_ast)
                .unwrap_or(Type::Void);

            methods.insert(
                sig.name.clone(),
                MethodSymbol {
                    name: sig.name.clone(),
                    params,
                    return_type,
                    is_public: true,
                    is_static: false,
                    is_override: false,
                    span: sig.span,
                },
            );
        }

        if self.symbols.interfaces.contains_key(&iface.name) {
            self.error(format!("Duplicate interface '{}'", iface.name), iface.span);
        } else {
            self.symbols.interfaces.insert(
                iface.name.clone(),
                InterfaceSymbol {
                    name: iface.name.clone(),
                    methods,
                    span: iface.span,
                },
            );
        }
    }

    fn register_class(&mut self, cls: &ClassDecl) {
        let mut fields = HashMap::new();
        let mut methods = HashMap::new();
        let mut constructor = None;
        let generic_params = cls.generic_params.clone();

        for member in &cls.members {
            match member {
                ClassMember::Field(f) => {
                    let mut ty = Type::from_ast(&f.ty);
                    if let Type::Class(ref name) = ty {
                        if generic_params.contains(name) {
                            ty = Type::GenericParam(name.clone());
                        }
                    }
                    let field = FieldSymbol {
                        name: f.name.clone(),
                        ty,
                        is_public: f.is_public,
                        is_static: f.is_static,
                        span: f.span,
                    };
                    if fields.contains_key(&f.name) {
                        self.error(
                            format!("Duplicate field '{}' in class '{}'", f.name, cls.name),
                            f.span,
                        );
                    } else {
                        fields.insert(f.name.clone(), field);
                    }
                }
                ClassMember::Constructor(c) => {
                    let params = c
                        .params
                        .iter()
                        .map(|p| {
                            let mut ty = Type::from_ast(&p.ty);
                            if let Type::Class(ref name) = ty {
                                if generic_params.contains(name) {
                                    ty = Type::GenericParam(name.clone());
                                }
                            }
                            (p.name.clone(), ty)
                        })
                        .collect();
                    constructor = Some(ConstructorSymbol {
                        params,
                        span: c.span,
                    });
                }
                ClassMember::Method(m) => {
                    let params = m
                        .func
                        .params
                        .iter()
                        .map(|p| {
                            let mut ty = Type::from_ast(&p.ty);
                            if let Type::Class(ref name) = ty {
                                if generic_params.contains(name) {
                                    ty = Type::GenericParam(name.clone());
                                }
                            }
                            (p.name.clone(), ty)
                        })
                        .collect();
                    let mut return_type = if m.func.is_async {
                        let inner = m
                            .func
                            .return_type
                            .as_ref()
                            .map(Type::from_ast)
                            .unwrap_or(Type::Void);
                        Type::GenericInstance("Task".to_string(), vec![inner])
                    } else {
                        m.func
                            .return_type
                            .as_ref()
                            .map(Type::from_ast)
                            .unwrap_or(Type::Void)
                    };
                    if let Type::Class(ref name) = return_type {
                        if generic_params.contains(name) {
                            return_type = Type::GenericParam(name.clone());
                        }
                    }

                    let method = MethodSymbol {
                        name: m.func.name.clone(),
                        params,
                        return_type,
                        is_public: m.is_public,
                        is_static: m.is_static,
                        is_override: m.is_override,
                        span: m.span,
                    };
                    if methods.contains_key(&m.func.name) {
                        self.error(
                            format!("Duplicate method '{}' in class '{}'", m.func.name, cls.name),
                            m.span,
                        );
                    } else {
                        methods.insert(m.func.name.clone(), method);
                    }
                }
            }
        }

        if self.symbols.classes.contains_key(&cls.name)
            && cls.name != "File"
            && cls.name != "Map"
            && cls.name != "Task"
            && cls.name != "Channel"
            && cls.name != "ThreadPool"
            && cls.name != "TcpListener"
            && cls.name != "TcpStream"
            && cls.name != "Arena"
        {
            self.error(format!("Duplicate class '{}'", cls.name), cls.span);
        } else {
            self.symbols.classes.insert(
                cls.name.clone(),
                ClassSymbol {
                    name: cls.name.clone(),
                    generic_params: cls.generic_params.clone(),
                    extends: cls.extends.clone(),
                    implements: cls.implements.clone(),
                    fields,
                    methods,
                    constructor,
                    span: cls.span,
                },
            );
        }
    }

    fn register_function(&mut self, func: &FunctionDecl) {
        let params = func
            .params
            .iter()
            .map(|p| (p.name.clone(), Type::from_ast(&p.ty)))
            .collect();
        let return_type = if func.is_async {
            let inner = func
                .return_type
                .as_ref()
                .map(Type::from_ast)
                .unwrap_or(Type::Void);
            Type::GenericInstance("Task".to_string(), vec![inner])
        } else {
            func.return_type
                .as_ref()
                .map(Type::from_ast)
                .unwrap_or(Type::Void)
        };

        if self.symbols.functions.contains_key(&func.name) {
            self.error(format!("Duplicate function '{}'", func.name), func.span);
        } else {
            self.symbols.functions.insert(
                func.name.clone(),
                FunctionSymbol {
                    name: func.name.clone(),
                    generic_params: func.generic_params.clone(),
                    params,
                    return_type,
                    span: func.span,
                },
            );
        }
    }

    // --- Pass 2: Interface Conformance ---
    fn check_interface_conformance(&mut self, cls: &ClassDecl) {
        let class_sym = match self.symbols.classes.get(&cls.name).cloned() {
            Some(c) => c,
            None => return,
        };

        for iface_name in &class_sym.implements {
            let iface_sym = match self.symbols.interfaces.get(iface_name).cloned() {
                Some(i) => i,
                None => {
                    self.error(
                        format!(
                            "Class '{}' implements undefined interface '{}'",
                            cls.name, iface_name
                        ),
                        cls.span,
                    );
                    continue;
                }
            };

            for (m_name, req_m) in &iface_sym.methods {
                match class_sym.methods.get(m_name) {
                    Some(cls_m) => {
                        if cls_m.return_type != req_m.return_type {
                            self.error(
                                format!(
                                    "Class '{}' implements interface '{}' but method '{}' return type '{}' does not match expected '{}'",
                                    cls.name, iface_name, m_name, cls_m.return_type, req_m.return_type
                                ),
                                cls_m.span,
                            );
                        }
                        if cls_m.params.len() != req_m.params.len() {
                            self.error(
                                format!(
                                    "Class '{}' implements interface '{}' but method '{}' has {} parameters (expected {})",
                                    cls.name, iface_name, m_name, cls_m.params.len(), req_m.params.len()
                                ),
                                cls_m.span,
                            );
                        } else {
                            for ((_, p_cls), (_, p_req)) in cls_m.params.iter().zip(&req_m.params) {
                                if p_cls != p_req {
                                    self.error(
                                        format!(
                                            "Class '{}' method '{}' parameter type '{}' does not match interface contract '{}'",
                                            cls.name, m_name, p_cls, p_req
                                        ),
                                        cls_m.span,
                                    );
                                }
                            }
                        }
                    }
                    None => {
                        self.error(
                            format!(
                                "Class '{}' promises to implement interface '{}' but is missing required method '{}'",
                                cls.name, iface_name, m_name
                            ),
                            cls.span,
                        );
                    }
                }
            }
        }
    }

    // --- Pass 3: Body & Expression Checking ---
    fn check_class_body(&mut self, cls: &ClassDecl) {
        self.current_class = Some(cls.name.clone());

        for member in &cls.members {
            match member {
                ClassMember::Constructor(c) => {
                    self.symbols.enter_scope();
                    // Inject parameters into scope
                    for p in &c.params {
                        let ty = Type::from_ast(&p.ty);
                        let _ = self.symbols.define_var(VariableSymbol {
                            name: p.name.clone(),
                            ty,
                            is_const: false,
                            span: p.span,
                        });
                    }
                    self.current_return_type = Some(Type::Void);
                    self.check_block(&c.body);
                    self.symbols.exit_scope();
                }
                ClassMember::Method(m) => {
                    self.check_function(&m.func);
                }
                ClassMember::Field(f) => {
                    if let Some(init) = &f.init {
                        let init_ty = self.check_expression(init);
                        let field_ty = Type::from_ast(&f.ty);
                        if !init_ty.is_assignable_to(&field_ty) {
                            self.error(
                                format!("Cannot initialize field '{}' of type '{}' with value of type '{}'", f.name, field_ty, init_ty),
                                f.span,
                            );
                        }
                    }
                }
            }
        }

        self.current_class = None;
    }

    fn check_function(&mut self, func: &FunctionDecl) {
        let return_type = func
            .return_type
            .as_ref()
            .map(Type::from_ast)
            .unwrap_or(Type::Void);

        self.current_return_type = Some(return_type);
        self.symbols.enter_scope();

        // Inject parameters into scope
        for p in &func.params {
            let ty = Type::from_ast(&p.ty);
            if let Err(existing) = self.symbols.define_var(VariableSymbol {
                name: p.name.clone(),
                ty,
                is_const: false,
                span: p.span,
            }) {
                self.error(
                    format!(
                        "Duplicate parameter name '{}' in function '{}'",
                        p.name, func.name
                    ),
                    existing.span,
                );
            }
        }

        self.check_block(&func.body);
        self.symbols.exit_scope();
        self.current_return_type = None;
    }

    fn check_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.check_statement(stmt);
        }
    }

    fn check_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                is_const,
                ty,
                name,
                init,
                span,
            } => {
                let declared_ty = Type::from_ast(ty);
                let init_ty = self.check_expression(init);

                if !init_ty.is_assignable_to(&declared_ty) {
                    self.error(
                        format!(
                            "Cannot assign value of type '{init_ty}' to variable '{name}' of type '{declared_ty}'"
                        ),
                        *span,
                    );
                }

                if let Err(existing) = self.symbols.define_var(VariableSymbol {
                    name: name.clone(),
                    ty: declared_ty,
                    is_const: *is_const,
                    span: *span,
                }) {
                    self.error(
                        format!("Variable '{name}' is already defined in this scope"),
                        existing.span,
                    );
                }
            }
            Stmt::Assignment {
                target,
                value,
                span,
            } => {
                let target_ty = self.check_expression(target);
                let value_ty = self.check_expression(value);

                if !value_ty.is_assignable_to(&target_ty) {
                    self.error(
                        format!(
                            "Cannot assign value of type '{value_ty}' to target of type '{target_ty}'"
                        ),
                        *span,
                    );
                }

                // Check const re-assignment
                if let Expr::Variable(name, _) = target {
                    if let Some(var) = self.symbols.lookup_var(name) {
                        if var.is_const {
                            self.error(
                                format!("Cannot reassign to constant variable '{name}'"),
                                *span,
                            );
                        }
                    }
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty = self.check_expression(condition);
                if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                    self.error(
                        format!("'if' condition must evaluate to 'bool', found '{cond_ty}'"),
                        *span,
                    );
                }
                self.symbols.enter_scope();
                self.check_block(then_branch);
                self.symbols.exit_scope();

                if let Some(else_b) = else_branch {
                    self.symbols.enter_scope();
                    self.check_block(else_b);
                    self.symbols.exit_scope();
                }
            }
            Stmt::While {
                condition,
                body,
                span,
            } => {
                let cond_ty = self.check_expression(condition);
                if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                    self.error(
                        format!("'while' condition must evaluate to 'bool', found '{cond_ty}'"),
                        *span,
                    );
                }
                self.symbols.enter_scope();
                self.check_block(body);
                self.symbols.exit_scope();
            }
            Stmt::For {
                init,
                condition,
                update,
                body,
                span,
            } => {
                self.symbols.enter_scope();
                if let Some(i) = init {
                    self.check_statement(i);
                }
                if let Some(c) = condition {
                    let cond_ty = self.check_expression(c);
                    if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                        self.error(
                            format!("'for' condition must evaluate to 'bool', found '{cond_ty}'"),
                            *span,
                        );
                    }
                }
                if let Some(u) = update {
                    self.check_statement(u);
                }
                self.check_block(body);
                self.symbols.exit_scope();
            }
            Stmt::ForIn {
                item_name,
                iterable,
                body,
                span,
            } => {
                let iter_ty = self.check_expression(iterable);
                let elem_ty = match iter_ty {
                    Type::Array(inner) => *inner,
                    Type::String => Type::String,
                    _ => {
                        self.error(
                            format!("'for..in' loop requires array or iterable, found '{iter_ty}'"),
                            *span,
                        );
                        Type::Unknown
                    }
                };

                self.symbols.enter_scope();
                if let Err(existing) = self.symbols.define_var(VariableSymbol {
                    name: item_name.clone(),
                    ty: elem_ty,
                    is_const: false,
                    span: *span,
                }) {
                    self.error(
                        format!("Variable '{item_name}' is already defined in scope"),
                        existing.span,
                    );
                }
                self.check_block(body);
                self.symbols.exit_scope();
            }
            Stmt::Return { value, span } => {
                let val_ty = value
                    .as_ref()
                    .map(|e| self.check_expression(e))
                    .unwrap_or(Type::Void);

                if let Some(expected) = &self.current_return_type {
                    if !val_ty.is_assignable_to(expected) {
                        self.error(
                            format!(
                                "Return type mismatch: expected '{expected}', found '{val_ty}'"
                            ),
                            *span,
                        );
                    }
                }
            }
            Stmt::Expr { expr, .. } => {
                self.check_expression(expr);
            }
            Stmt::Throw { expr, .. } => {
                self.check_expression(expr);
            }
            Stmt::TryCatch {
                try_block,
                catch_param,
                catch_block,
                finally_block,
                span,
            } => {
                self.symbols.enter_scope();
                self.check_block(try_block);
                self.symbols.exit_scope();

                if let Some(cb) = catch_block {
                    self.symbols.enter_scope();
                    if let Some(cp) = catch_param {
                        if let Err(existing) = self.symbols.define_var(VariableSymbol {
                            name: cp.clone(),
                            ty: Type::String,
                            is_const: false,
                            span: *span,
                        }) {
                            self.error(
                                format!("Variable '{cp}' is already defined in catch scope"),
                                existing.span,
                            );
                        }
                    }
                    self.check_block(cb);
                    self.symbols.exit_scope();
                }

                if let Some(fb) = finally_block {
                    self.symbols.enter_scope();
                    self.check_block(fb);
                    self.symbols.exit_scope();
                }
            }
        }
    }

    fn check_expression(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Integer(_, _) => Type::Int,
            Expr::Float(_, _) => Type::Float,
            Expr::StringLit(_, _) => Type::String,
            Expr::BoolLit(_, _) => Type::Bool,
            Expr::Variable(name, span) => {
                if let Some(var) = self.symbols.lookup_var(name) {
                    var.ty.clone()
                } else if self.symbols.classes.contains_key(name) {
                    Type::Class(name.clone())
                } else if let Some(func) = self.symbols.functions.get(name) {
                    Type::Function {
                        params: func.params.iter().map(|(_, ty)| ty.clone()).collect(),
                        return_type: Box::new(func.return_type.clone()),
                    }
                } else if self.current_class.is_some() {
                    // Check if it's a field on this
                    let cls_name = self.current_class.as_ref().unwrap();
                    if let Some(cls) = self.symbols.classes.get(cls_name) {
                        if let Some(field) = cls.fields.get(name) {
                            return field.ty.clone();
                        }
                    }
                    self.error(format!("Undefined variable or field '{name}'"), *span);
                    Type::Unknown
                } else {
                    self.error(format!("Undefined variable '{name}'"), *span);
                    Type::Unknown
                }
            }
            Expr::This(span) => {
                if let Some(cls_name) = &self.current_class {
                    Type::Class(cls_name.clone())
                } else {
                    self.error(
                        "'this' can only be used inside a class method or constructor",
                        *span,
                    );
                    Type::Unknown
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let left_ty = self.check_expression(left);
                let right_ty = self.check_expression(right);

                match op {
                    BinaryOp::Add => {
                        if left_ty == Type::Int && right_ty == Type::Int {
                            Type::Int
                        } else if left_ty == Type::Float && right_ty == Type::Float {
                            Type::Float
                        } else if left_ty == Type::String || right_ty == Type::String {
                            Type::String // String concatenation!
                        } else if left_ty == Type::Unknown || right_ty == Type::Unknown {
                            if left_ty == Type::String || right_ty == Type::String {
                                Type::String
                            } else if left_ty == Type::Float || right_ty == Type::Float {
                                Type::Float
                            } else {
                                Type::Int
                            }
                        } else {
                            self.error(
                                format!("Cannot add types '{left_ty}' and '{right_ty}'"),
                                *span,
                            );
                            Type::Unknown
                        }
                    }
                    BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                        if left_ty == Type::Int && right_ty == Type::Int {
                            Type::Int
                        } else if left_ty == Type::Float && right_ty == Type::Float {
                            Type::Float
                        } else if left_ty == Type::Unknown || right_ty == Type::Unknown {
                            if left_ty == Type::Int || right_ty == Type::Int {
                                Type::Int
                            } else if left_ty == Type::Float || right_ty == Type::Float {
                                Type::Float
                            } else {
                                Type::Unknown
                            }
                        } else {
                            self.error(format!("Arithmetic operation requires numeric types, found '{left_ty}' and '{right_ty}'"), *span);
                            Type::Unknown
                        }
                    }
                    BinaryOp::EqualEqual | BinaryOp::NotEqual => {
                        if left_ty.is_assignable_to(&right_ty)
                            || right_ty.is_assignable_to(&left_ty)
                        {
                            Type::Bool
                        } else {
                            self.error(
                                format!(
                                    "Cannot compare equality between unrelated types '{left_ty}' and '{right_ty}'"
                                ),
                                *span,
                            );
                            Type::Bool
                        }
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if (left_ty.is_numeric() || left_ty == Type::Unknown)
                            && (right_ty.is_numeric() || right_ty == Type::Unknown)
                        {
                            Type::Bool
                        } else {
                            self.error(
                                format!(
                                    "Ordering comparison requires numbers, found '{left_ty}' and '{right_ty}'"
                                ),
                                *span,
                            );
                            Type::Bool
                        }
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if (left_ty == Type::Bool || left_ty == Type::Unknown)
                            && (right_ty == Type::Bool || right_ty == Type::Unknown)
                        {
                            Type::Bool
                        } else {
                            self.error(format!("Logical operator requires boolean operands, found '{left_ty}' and '{right_ty}'"), *span);
                            Type::Bool
                        }
                    }
                    BinaryOp::Mod
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => {
                        if (left_ty == Type::Int || left_ty == Type::Unknown)
                            && (right_ty == Type::Int || right_ty == Type::Unknown)
                        {
                            Type::Int
                        } else {
                            self.error(format!("Bitwise/modulo operation requires integer operands, found '{left_ty}' and '{right_ty}'"), *span);
                            Type::Int
                        }
                    }
                }
            }
            Expr::Call { callee, args, span } => {
                match callee.as_ref() {
                    Expr::Variable(fn_name, fn_span) => {
                        // Built-in functions
                        if fn_name == "println" || fn_name == "print" {
                            for arg in args {
                                self.check_expression(arg);
                            }
                            return Type::Void;
                        }
                        if fn_name == "readln" {
                            return Type::String;
                        }
                        if fn_name == "abs" {
                            if let Some(arg) = args.first() {
                                self.check_expression(arg);
                            }
                            return Type::Int;
                        }
                        if fn_name == "min" || fn_name == "max" {
                            if args.is_empty() {
                                self.error(
                                    format!("'{fn_name}' requires at least 1 argument"),
                                    *span,
                                );
                            } else if args.len() == 1 {
                                let arg_ty = self.check_expression(&args[0]);
                                if let Type::Array(inner) = &arg_ty {
                                    if !inner.is_numeric() && **inner != Type::Unknown {
                                        self.error(
                                            format!(
                                                "'{fn_name}' requires numeric array elements, found '{inner}'"
                                            ),
                                            args[0].span(),
                                        );
                                    }
                                }
                            } else {
                                for arg in args {
                                    let arg_ty = self.check_expression(arg);
                                    if !arg_ty.is_numeric() && arg_ty != Type::Unknown {
                                        self.error(
                                            format!(
                                                "'{fn_name}' expects numeric arguments, found '{arg_ty}'"
                                            ),
                                            arg.span(),
                                        );
                                    }
                                }
                            }
                            return Type::Int;
                        }

                        if let Some(func) = self.symbols.functions.get(fn_name).cloned() {
                            if func.params.len() != args.len() {
                                self.error(
                                    format!(
                                        "Function '{}' expects {} arguments, but got {}",
                                        fn_name,
                                        func.params.len(),
                                        args.len()
                                    ),
                                    *span,
                                );
                            } else {
                                for (arg_expr, (_, param_ty)) in args.iter().zip(&func.params) {
                                    let arg_ty = self.check_expression(arg_expr);
                                    if !arg_ty.is_assignable_to(param_ty) {
                                        self.error(
                                            format!("Argument type '{arg_ty}' does not match parameter type '{param_ty}'"),
                                            arg_expr.span(),
                                        );
                                    }
                                }
                            }
                            func.return_type
                        } else if let Some(var) = self.symbols.lookup_var(fn_name).cloned() {
                            for arg in args {
                                self.check_expression(arg);
                            }
                            match var.ty {
                                Type::Function { return_type, .. } => *return_type,
                                _ => Type::Unknown,
                            }
                        } else {
                            self.error(format!("Call to undefined function '{fn_name}'"), *fn_span);
                            Type::Unknown
                        }
                    }
                    Expr::MemberAccess {
                        object,
                        member,
                        span: m_span,
                    } => {
                        // Check for static Math namespace
                        if let Expr::Variable(obj_name, _) = object.as_ref() {
                            if obj_name == "Math" {
                                if member == "min" || member == "max" {
                                    if args.is_empty() {
                                        self.error(
                                            format!("Math.{member} requires at least 1 argument"),
                                            *span,
                                        );
                                        return Type::Int;
                                    }
                                    if args.len() == 1 {
                                        let arg_ty = self.check_expression(&args[0]);
                                        if let Type::Array(inner) = &arg_ty {
                                            if !inner.is_numeric() && **inner != Type::Unknown {
                                                self.error(format!("Math.{member} requires numeric array elements, found '{inner}'"), args[0].span());
                                            }
                                        } else if !arg_ty.is_numeric() && arg_ty != Type::Unknown {
                                            self.error(format!("Math.{member} expects numeric arguments or array, found '{arg_ty}'"), args[0].span());
                                        }
                                    } else {
                                        for arg in args {
                                            let arg_ty = self.check_expression(arg);
                                            if !arg_ty.is_numeric() && arg_ty != Type::Unknown {
                                                self.error(format!("Math.{member} expects numeric arguments, found '{arg_ty}'"), arg.span());
                                            }
                                        }
                                    }
                                    return Type::Int;
                                }
                                if member == "abs" {
                                    if args.len() != 1 {
                                        self.error(
                                            format!(
                                                "Math.abs expects 1 argument, found {}",
                                                args.len()
                                            ),
                                            *span,
                                        );
                                    }
                                    if let Some(arg) = args.first() {
                                        let arg_ty = self.check_expression(arg);
                                        if !arg_ty.is_numeric() && arg_ty != Type::Unknown {
                                            self.error(
                                                format!(
                                                    "Math.abs expects numeric argument, found '{arg_ty}'"
                                                ),
                                                arg.span(),
                                            );
                                        }
                                    }
                                    return Type::Int;
                                }
                                if member == "clamp" {
                                    if args.len() != 3 {
                                        self.error(format!("Math.clamp expects 3 arguments (val, min, max), found {}", args.len()), *span);
                                    }
                                    for arg in args {
                                        let arg_ty = self.check_expression(arg);
                                        if !arg_ty.is_numeric() && arg_ty != Type::Unknown {
                                            self.error(format!("Math.clamp expects numeric arguments, found '{arg_ty}'"), arg.span());
                                        }
                                    }
                                    return Type::Int;
                                }
                                if member == "pow" {
                                    if args.len() != 2 {
                                        self.error(format!("Math.pow expects 2 arguments (base, exp), found {}", args.len()), *span);
                                    }
                                    for arg in args {
                                        let arg_ty = self.check_expression(arg);
                                        if !arg_ty.is_numeric() && arg_ty != Type::Unknown {
                                            self.error(format!("Math.pow expects numeric arguments, found '{arg_ty}'"), arg.span());
                                        }
                                    }
                                    return Type::Int;
                                }
                                if member == "sqrt" {
                                    if args.len() != 1 {
                                        self.error(
                                            format!(
                                                "Math.sqrt expects 1 argument, found {}",
                                                args.len()
                                            ),
                                            *span,
                                        );
                                    }
                                    if let Some(arg) = args.first() {
                                        let arg_ty = self.check_expression(arg);
                                        if !arg_ty.is_numeric() && arg_ty != Type::Unknown {
                                            self.error(format!("Math.sqrt expects numeric argument, found '{arg_ty}'"), arg.span());
                                        }
                                    }
                                    return Type::Int;
                                }
                                self.error(format!("Unknown method 'Math.{member}'"), *m_span);
                                return Type::Unknown;
                            }

                            // Check for static File namespace
                            if obj_name == "File" {
                                match member.as_str() {
                                    "readText" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "File.readText(path) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("File.readText expects string path, found '{a_ty}'"), args[0].span());
                                            }
                                        }
                                        return Type::String;
                                    }
                                    "writeText" => {
                                        if args.len() != 2 {
                                            self.error(
                                                "File.writeText(path, content) expects 2 arguments",
                                                *span,
                                            );
                                        } else {
                                            for a in args {
                                                let a_ty = self.check_expression(a);
                                                if !a_ty.is_assignable_to(&Type::String)
                                                    && a_ty != Type::Unknown
                                                {
                                                    self.error(format!("File.writeText expects string arguments, found '{a_ty}'"), a.span());
                                                }
                                            }
                                        }
                                        return Type::Bool;
                                    }
                                    "appendText" => {
                                        if args.len() != 2 {
                                            self.error("File.appendText(path, content) expects 2 arguments", *span);
                                        } else {
                                            for a in args {
                                                let a_ty = self.check_expression(a);
                                                if !a_ty.is_assignable_to(&Type::String)
                                                    && a_ty != Type::Unknown
                                                {
                                                    self.error(format!("File.appendText expects string arguments, found '{a_ty}'"), a.span());
                                                }
                                            }
                                        }
                                        return Type::Bool;
                                    }
                                    "exists" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "File.exists(path) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("File.exists expects string path, found '{a_ty}'"), args[0].span());
                                            }
                                        }
                                        return Type::Bool;
                                    }
                                    "delete" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "File.delete(path) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("File.delete expects string path, found '{a_ty}'"), args[0].span());
                                            }
                                        }
                                        return Type::Bool;
                                    }
                                    "size" => {
                                        if args.len() != 1 {
                                            self.error("File.size(path) expects 1 argument", *span);
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(
                                                    format!(
                                                        "File.size expects string path, found '{a_ty}'"
                                                    ),
                                                    args[0].span(),
                                                );
                                            }
                                        }
                                        return Type::Int;
                                    }
                                    "readLines" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "File.readLines(path) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("File.readLines expects string path, found '{a_ty}'"), args[0].span());
                                            }
                                        }
                                        return Type::Array(Box::new(Type::String));
                                    }
                                    _ => {
                                        self.error(
                                            format!("Unknown method 'File.{member}'"),
                                            *m_span,
                                        );
                                        return Type::Unknown;
                                    }
                                }
                            }

                            // Check for static System namespace
                            if obj_name == "System" {
                                match member.as_str() {
                                    "args" => {
                                        if !args.is_empty() {
                                            self.error("System.args() expects 0 arguments", *span);
                                        }
                                        return Type::Array(Box::new(Type::String));
                                    }
                                    "free" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "System.free(ptr) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            self.check_expression(&args[0]);
                                        }
                                        return Type::Void;
                                    }
                                    "exit" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "System.exit(code) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::Int)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(
                                                    format!(
                                                        "System.exit expects int code, found '{a_ty}'"
                                                    ),
                                                    args[0].span(),
                                                );
                                            }
                                        }
                                        return Type::Void;
                                    }
                                    "exec" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "System.exec(cmd) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("System.exec expects string command, found '{a_ty}'"), args[0].span());
                                            }
                                        }
                                        return Type::Int;
                                    }
                                    "getEnv" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "System.getEnv(key) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("System.getEnv expects string key, found '{a_ty}'"), args[0].span());
                                            }
                                        }
                                        return Type::String;
                                    }
                                    "timeMs" | "currentTimeMillis" => {
                                        if !args.is_empty() {
                                            self.error(
                                                "System.timeMs() expects 0 arguments",
                                                *span,
                                            );
                                        }
                                        return Type::Int;
                                    }
                                    "println" => {
                                        for arg in args {
                                            self.check_expression(arg);
                                        }
                                        return Type::Void;
                                    }
                                    "print" => {
                                        for arg in args {
                                            self.check_expression(arg);
                                        }
                                        return Type::Void;
                                    }
                                    _ => {
                                        self.error(
                                            format!("Unknown method 'System.{member}'"),
                                            *m_span,
                                        );
                                        return Type::Unknown;
                                    }
                                }
                            }

                            if obj_name == "Json" {
                                match member.as_str() {
                                    "parse" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "Json.parse(json_str) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("Json.parse expects string argument, found '{a_ty}'"), args[0].span());
                                            }
                                        }
                                        return Type::GenericInstance(
                                            "Map".to_string(),
                                            vec![Type::String, Type::Unknown],
                                        );
                                    }
                                    "stringify" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "Json.stringify(val) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            self.check_expression(&args[0]);
                                        }
                                        return Type::String;
                                    }
                                    _ => {
                                        self.error(
                                            format!("Unknown method 'Json.{member}'"),
                                            *m_span,
                                        );
                                        return Type::Unknown;
                                    }
                                }
                            }

                            if obj_name == "String" {
                                match member.as_str() {
                                    "fromCharCode" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "String.fromCharCode(code) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            let a_ty = self.check_expression(&args[0]);
                                            if !a_ty.is_assignable_to(&Type::Int)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("String.fromCharCode expects int, found '{a_ty}'"), args[0].span());
                                            }
                                        }
                                        return Type::String;
                                    }
                                    _ => {
                                        self.error(
                                            format!("Unknown method 'String.{member}'"),
                                            *m_span,
                                        );
                                        return Type::Unknown;
                                    }
                                }
                            }

                            // Check for static Schema methods
                            if obj_name == "Schema" {
                                match member.as_str() {
                                    "describe" => {
                                        if args.len() != 1 {
                                            self.error(
                                                "Schema.describe(target) expects 1 argument",
                                                *span,
                                            );
                                        } else {
                                            self.check_expression(&args[0]);
                                        }
                                        return Type::String;
                                    }
                                    _ => {
                                        self.error(
                                            format!("Unknown method 'Schema.{member}'"),
                                            *m_span,
                                        );
                                        return Type::Unknown;
                                    }
                                }
                            }

                            // Check user-defined namespace functions
                            let ns_fn_name = format!("{obj_name}_{member}");
                            if let Some(func) = self.symbols.functions.get(&ns_fn_name).cloned() {
                                if func.params.len() != args.len() {
                                    self.error(
                                        format!(
                                            "Function '{}.{}' expects {} arguments, but got {}",
                                            obj_name,
                                            member,
                                            func.params.len(),
                                            args.len()
                                        ),
                                        *span,
                                    );
                                } else {
                                    for (arg_expr, (_, param_ty)) in args.iter().zip(&func.params) {
                                        let arg_ty = self.check_expression(arg_expr);
                                        if !arg_ty.is_assignable_to(param_ty) {
                                            self.error(
                                                format!("Argument type '{arg_ty}' does not match parameter type '{param_ty}'"),
                                                arg_expr.span(),
                                            );
                                        }
                                    }
                                }
                                return func.return_type;
                            }

                            // Check user-defined class static methods
                            if let Some(cls) = self.symbols.classes.get(obj_name).cloned() {
                                if let Some(m) = cls.methods.get(member) {
                                    if m.is_static {
                                        if obj_name == "Task" && member == "spawn" {
                                            if args.is_empty() || args.len() > 2 {
                                                self.error(
                                                    format!("Static method 'Task.spawn' expects 1 or 2 arguments, but got {}", args.len()),
                                                    *span,
                                                );
                                            }
                                            for a in args {
                                                self.check_expression(a);
                                            }
                                            return m.return_type.clone();
                                        }
                                        if obj_name == "Channel" && member == "select" {
                                            if args.len() != 1 {
                                                self.error(
                                                    format!("Static method 'Channel.select' expects 1 argument, but got {}", args.len()),
                                                    *span,
                                                );
                                            } else {
                                                self.check_expression(&args[0]);
                                            }
                                            return Type::Int;
                                        }
                                        if obj_name == "Arena" && member == "create" {
                                            if args.len() > 1 {
                                                self.error(format!("Static method 'Arena.create' expects 0 or 1 argument, but got {}", args.len()), *span);
                                            } else if !args.is_empty() {
                                                let arg_ty = self.check_expression(&args[0]);
                                                if !arg_ty.is_assignable_to(&Type::Int) {
                                                    self.error(format!("Argument to 'Arena.create' must be an int, found '{arg_ty}'"), args[0].span());
                                                }
                                            }
                                            return m.return_type.clone();
                                        }
                                        if m.params.len() != args.len() {
                                            self.error(
                                                format!("Static method '{}.{}' expects {} arguments, but got {}", obj_name, member, m.params.len(), args.len()),
                                                *span,
                                            );
                                        } else {
                                            for (arg_expr, (_, param_ty)) in
                                                args.iter().zip(&m.params)
                                            {
                                                let arg_ty = self.check_expression(arg_expr);
                                                if !arg_ty.is_assignable_to(param_ty) {
                                                    self.error(
                                                        format!("Argument type '{arg_ty}' does not match parameter type '{param_ty}'"),
                                                        arg_expr.span(),
                                                    );
                                                }
                                            }
                                        }
                                        return m.return_type.clone();
                                    }
                                }
                            }
                        }

                        let obj_ty = self.check_expression(object);

                        if obj_ty == Type::String {
                            match member.as_str() {
                                "length" | "len" => {
                                    if !args.is_empty() {
                                        self.error("String.length() expects 0 arguments", *span);
                                    }
                                    return Type::Int;
                                }
                                "isEmpty" => {
                                    if !args.is_empty() {
                                        self.error("String.isEmpty() expects 0 arguments", *span);
                                    }
                                    return Type::Bool;
                                }
                                "contains" => {
                                    if args.len() != 1 {
                                        self.error(
                                            "String.contains(sub) expects 1 argument",
                                            *span,
                                        );
                                    } else {
                                        let a_ty = self.check_expression(&args[0]);
                                        if !a_ty.is_assignable_to(&Type::String)
                                            && a_ty != Type::Unknown
                                        {
                                            self.error(format!("String.contains expects string argument, found '{a_ty}'"), args[0].span());
                                        }
                                    }
                                    return Type::Bool;
                                }
                                "indexOf" => {
                                    if args.len() != 1 {
                                        self.error("String.indexOf(sub) expects 1 argument", *span);
                                    } else {
                                        let a_ty = self.check_expression(&args[0]);
                                        if !a_ty.is_assignable_to(&Type::String)
                                            && a_ty != Type::Unknown
                                        {
                                            self.error(format!("String.indexOf expects string argument, found '{a_ty}'"), args[0].span());
                                        }
                                    }
                                    return Type::Int;
                                }
                                "startsWith" => {
                                    if args.len() != 1 {
                                        self.error(
                                            "String.startsWith(prefix) expects 1 argument",
                                            *span,
                                        );
                                    } else {
                                        let a_ty = self.check_expression(&args[0]);
                                        if !a_ty.is_assignable_to(&Type::String)
                                            && a_ty != Type::Unknown
                                        {
                                            self.error(format!("String.startsWith expects string argument, found '{a_ty}'"), args[0].span());
                                        }
                                    }
                                    return Type::Bool;
                                }
                                "endsWith" => {
                                    if args.len() != 1 {
                                        self.error(
                                            "String.endsWith(suffix) expects 1 argument",
                                            *span,
                                        );
                                    } else {
                                        let a_ty = self.check_expression(&args[0]);
                                        if !a_ty.is_assignable_to(&Type::String)
                                            && a_ty != Type::Unknown
                                        {
                                            self.error(format!("String.endsWith expects string argument, found '{a_ty}'"), args[0].span());
                                        }
                                    }
                                    return Type::Bool;
                                }
                                "toUpper" | "toUpperCase" => {
                                    if !args.is_empty() {
                                        self.error(
                                            format!("String.{member}() expects 0 arguments"),
                                            *span,
                                        );
                                    }
                                    return Type::String;
                                }
                                "toLower" | "toLowerCase" => {
                                    if !args.is_empty() {
                                        self.error(
                                            format!("String.{member}() expects 0 arguments"),
                                            *span,
                                        );
                                    }
                                    return Type::String;
                                }
                                "trim" => {
                                    if !args.is_empty() {
                                        self.error("String.trim() expects 0 arguments", *span);
                                    }
                                    return Type::String;
                                }
                                "slice" | "substring" => {
                                    if args.len() != 2 {
                                        self.error(
                                            format!(
                                                "String.{member}(start, end) expects 2 arguments"
                                            ),
                                            *span,
                                        );
                                    } else {
                                        for a in args {
                                            let a_ty = self.check_expression(a);
                                            if !a_ty.is_numeric() && a_ty != Type::Unknown {
                                                self.error(format!("String.{member} expects numeric index, found '{a_ty}'"), a.span());
                                            }
                                        }
                                    }
                                    return Type::String;
                                }
                                "charAt" => {
                                    if args.len() != 1 {
                                        self.error(
                                            "String.charAt(index) expects 1 argument",
                                            *span,
                                        );
                                    } else {
                                        let a_ty = self.check_expression(&args[0]);
                                        if !a_ty.is_numeric() && a_ty != Type::Unknown {
                                            self.error(format!("String.charAt expects numeric index, found '{a_ty}'"), args[0].span());
                                        }
                                    }
                                    return Type::String;
                                }
                                "charCodeAt" => {
                                    if args.len() != 1 {
                                        self.error(
                                            "String.charCodeAt(index) expects 1 argument",
                                            *span,
                                        );
                                    } else {
                                        let a_ty = self.check_expression(&args[0]);
                                        if !a_ty.is_numeric() && a_ty != Type::Unknown {
                                            self.error(format!("String.charCodeAt expects numeric index, found '{a_ty}'"), args[0].span());
                                        }
                                    }
                                    return Type::Int;
                                }
                                "replace" => {
                                    if args.len() != 2 {
                                        self.error("String.replace(target, replacement) expects 2 arguments", *span);
                                    } else {
                                        for a in args {
                                            let a_ty = self.check_expression(a);
                                            if !a_ty.is_assignable_to(&Type::String)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("String.replace expects string arguments, found '{a_ty}'"), a.span());
                                            }
                                        }
                                    }
                                    return Type::String;
                                }
                                "split" => {
                                    if args.len() != 1 {
                                        self.error(
                                            "String.split(delimiter) expects 1 argument",
                                            *span,
                                        );
                                    } else {
                                        let a_ty = self.check_expression(&args[0]);
                                        if !a_ty.is_assignable_to(&Type::String)
                                            && a_ty != Type::Unknown
                                        {
                                            self.error(format!("String.split expects string delimiter, found '{a_ty}'"), args[0].span());
                                        }
                                    }
                                    return Type::Array(Box::new(Type::String));
                                }
                                "toInt" => {
                                    if !args.is_empty() {
                                        self.error("String.toInt() expects 0 arguments", *span);
                                    }
                                    return Type::Int;
                                }
                                _ => {
                                    self.error(
                                        format!(
                                            "No method named '{member}' found on type 'string'"
                                        ),
                                        *m_span,
                                    );
                                    return Type::Unknown;
                                }
                            }
                        }

                        if let Type::Array(elem_ty) = &obj_ty {
                            match member.as_str() {
                                "push" => {
                                    if args.len() != 1 {
                                        self.error("Array.push(val) expects 1 argument", *span);
                                    } else {
                                        self.check_expression(&args[0]);
                                    }
                                    return Type::Int;
                                }
                                "length" | "len" => {
                                    if !args.is_empty() {
                                        self.error("Array.length() expects 0 arguments", *span);
                                    }
                                    return Type::Int;
                                }
                                "isEmpty" => {
                                    if !args.is_empty() {
                                        self.error("Array.isEmpty() expects 0 arguments", *span);
                                    }
                                    return Type::Bool;
                                }
                                "join" => {
                                    if args.len() != 1 {
                                        self.error(
                                            "Array.join(separator) expects 1 argument",
                                            *span,
                                        );
                                    } else {
                                        let a_ty = self.check_expression(&args[0]);
                                        if !a_ty.is_assignable_to(&Type::String)
                                            && a_ty != Type::Unknown
                                        {
                                            self.error(format!("Array.join expects string separator, found '{a_ty}'"), args[0].span());
                                        }
                                    }
                                    return Type::String;
                                }
                                "map" => {
                                    if args.len() != 1 {
                                        self.error("Array.map(fn) expects 1 argument", *span);
                                        return Type::Array(Box::new(Type::Unknown));
                                    }
                                    let fn_ty = self.check_expression(&args[0]);
                                    let ret_elem = match fn_ty {
                                        Type::Function { return_type, .. } => *return_type,
                                        _ => Type::Unknown,
                                    };
                                    return Type::Array(Box::new(ret_elem));
                                }
                                "filter" => {
                                    if args.len() != 1 {
                                        self.error(
                                            "Array.filter(predicate) expects 1 argument",
                                            *span,
                                        );
                                    } else {
                                        self.check_expression(&args[0]);
                                    }
                                    return Type::Array((*elem_ty).clone());
                                }
                                "reduce" => {
                                    if args.len() != 2 {
                                        self.error(
                                            "Array.reduce(reducer, initial) expects 2 arguments",
                                            *span,
                                        );
                                        return Type::Unknown;
                                    }
                                    self.check_expression(&args[0]);
                                    let init_ty = self.check_expression(&args[1]);
                                    return init_ty;
                                }
                                "forEach" => {
                                    if args.len() != 1 {
                                        self.error("Array.forEach(fn) expects 1 argument", *span);
                                    } else {
                                        self.check_expression(&args[0]);
                                    }
                                    return Type::Void;
                                }
                                _ => {
                                    self.error(
                                        format!("No method named '{member}' found on array"),
                                        *m_span,
                                    );
                                    return Type::Unknown;
                                }
                            }
                        }

                        if let Type::GenericInstance(cls_name, generic_args) = &obj_ty {
                            if cls_name == "Map" {
                                match member.as_str() {
                                    "set" => {
                                        if args.len() != 2 {
                                            self.error(
                                                "Map.set(key, value) expects 2 arguments",
                                                *span,
                                            );
                                        } else {
                                            let k_ty = self.check_expression(&args[0]);
                                            let v_ty = self.check_expression(&args[1]);
                                            let exp_k =
                                                generic_args.first().unwrap_or(&Type::String);
                                            let exp_v =
                                                generic_args.get(1).unwrap_or(&Type::Unknown);
                                            if !k_ty.is_assignable_to(exp_k) {
                                                self.error(format!("Map.set key type '{k_ty}' does not match expected '{exp_k}'"), args[0].span());
                                            }
                                            if !v_ty.is_assignable_to(exp_v) {
                                                self.error(format!("Map.set value type '{v_ty}' does not match expected '{exp_v}'"), args[1].span());
                                            }
                                        }
                                        return Type::Void;
                                    }
                                    "get" => {
                                        if args.len() != 1 {
                                            self.error("Map.get(key) expects 1 argument", *span);
                                        } else {
                                            let k_ty = self.check_expression(&args[0]);
                                            let exp_k =
                                                generic_args.first().unwrap_or(&Type::String);
                                            if !k_ty.is_assignable_to(exp_k) {
                                                self.error(format!("Map.get key type '{k_ty}' does not match expected '{exp_k}'"), args[0].span());
                                            }
                                        }
                                        return generic_args
                                            .get(1)
                                            .cloned()
                                            .unwrap_or(Type::Unknown);
                                    }
                                    "has" => {
                                        if args.len() != 1 {
                                            self.error("Map.has(key) expects 1 argument", *span);
                                        } else {
                                            self.check_expression(&args[0]);
                                        }
                                        return Type::Bool;
                                    }
                                    "remove" | "delete" => {
                                        if args.len() != 1 {
                                            self.error("Map.remove(key) expects 1 argument", *span);
                                        } else {
                                            self.check_expression(&args[0]);
                                        }
                                        return Type::Bool;
                                    }
                                    "len" | "size" => {
                                        if !args.is_empty() {
                                            self.error("Map.len() expects 0 arguments", *span);
                                        }
                                        return Type::Int;
                                    }
                                    "keys" => {
                                        if !args.is_empty() {
                                            self.error("Map.keys() expects 0 arguments", *span);
                                        }
                                        let k_ty =
                                            generic_args.first().cloned().unwrap_or(Type::String);
                                        return Type::Array(Box::new(k_ty));
                                    }
                                    "values" => {
                                        if !args.is_empty() {
                                            self.error("Map.values() expects 0 arguments", *span);
                                        }
                                        let v_ty =
                                            generic_args.get(1).cloned().unwrap_or(Type::Unknown);
                                        return Type::Array(Box::new(v_ty));
                                    }
                                    "clear" => {
                                        if !args.is_empty() {
                                            self.error("Map.clear() expects 0 arguments", *span);
                                        }
                                        return Type::Void;
                                    }
                                    _ => {
                                        self.error(
                                            format!("No method named '{member}' found on Map"),
                                            *m_span,
                                        );
                                        return Type::Unknown;
                                    }
                                }
                            }

                            if cls_name == "Task"
                                && (member == "await" || member == "join" || member == "wait")
                            {
                                if !args.is_empty() {
                                    self.error(
                                        format!("Task.{member}() expects 0 arguments"),
                                        *span,
                                    );
                                }
                                return generic_args.first().cloned().unwrap_or(Type::Unknown);
                            }

                            if let Some(cls) = self.symbols.classes.get(cls_name).cloned() {
                                if let Some(m) = cls.methods.get(member) {
                                    let mut mapping = HashMap::new();
                                    for (p, a) in cls.generic_params.iter().zip(generic_args.iter())
                                    {
                                        mapping.insert(p.clone(), a.clone());
                                    }
                                    if m.params.len() != args.len() {
                                        self.error(
                                            format!("Method '{}' on class '{}' expects {} arguments, but got {}", member, cls_name, m.params.len(), args.len()),
                                            *span,
                                        );
                                    } else {
                                        for (arg_expr, (_, param_ty)) in args.iter().zip(&m.params)
                                        {
                                            let arg_ty = self.check_expression(arg_expr);
                                            let exp_ty =
                                                substitute_type_mapping(param_ty, &mapping);
                                            if !arg_ty.is_assignable_to(&exp_ty) {
                                                self.error(
                                                    format!("Argument type '{arg_ty}' does not match method parameter type '{exp_ty}'"),
                                                    arg_expr.span(),
                                                );
                                            }
                                        }
                                    }
                                    return substitute_type_mapping(&m.return_type, &mapping);
                                } else if let Some(field) = cls.fields.get(member) {
                                    for a in args {
                                        self.check_expression(a);
                                    }
                                    match &field.ty {
                                        Type::Function { return_type, .. } => {
                                            return *return_type.clone()
                                        }
                                        _ => return Type::Unknown,
                                    }
                                } else {
                                    self.error(
                                        format!(
                                            "No method named '{member}' found on class '{cls_name}'"
                                        ),
                                        *m_span,
                                    );
                                    return Type::Unknown;
                                }
                            }
                        }

                        if let Type::Class(cls_name) = &obj_ty {
                            if cls_name == "Map" {
                                match member.as_str() {
                                    "set" => {
                                        if args.len() != 2 {
                                            self.error(
                                                "Map.set(key, value) expects 2 arguments",
                                                *span,
                                            );
                                        } else {
                                            self.check_expression(&args[0]);
                                            self.check_expression(&args[1]);
                                        }
                                        return Type::Void;
                                    }
                                    "get" => {
                                        if args.len() != 1 {
                                            self.error("Map.get(key) expects 1 argument", *span);
                                        } else {
                                            self.check_expression(&args[0]);
                                        }
                                        return Type::Unknown;
                                    }
                                    "has" => {
                                        if args.len() != 1 {
                                            self.error("Map.has(key) expects 1 argument", *span);
                                        } else {
                                            self.check_expression(&args[0]);
                                        }
                                        return Type::Bool;
                                    }
                                    "remove" | "delete" => {
                                        if args.len() != 1 {
                                            self.error("Map.remove(key) expects 1 argument", *span);
                                        } else {
                                            self.check_expression(&args[0]);
                                        }
                                        return Type::Bool;
                                    }
                                    "len" | "size" => {
                                        if !args.is_empty() {
                                            self.error("Map.len() expects 0 arguments", *span);
                                        }
                                        return Type::Int;
                                    }
                                    "keys" => {
                                        if !args.is_empty() {
                                            self.error("Map.keys() expects 0 arguments", *span);
                                        }
                                        return Type::Array(Box::new(Type::String));
                                    }
                                    "values" => {
                                        if !args.is_empty() {
                                            self.error("Map.values() expects 0 arguments", *span);
                                        }
                                        return Type::Array(Box::new(Type::Unknown));
                                    }
                                    "clear" => {
                                        if !args.is_empty() {
                                            self.error("Map.clear() expects 0 arguments", *span);
                                        }
                                        return Type::Void;
                                    }
                                    _ => {
                                        self.error(
                                            format!("No method named '{member}' found on Map"),
                                            *m_span,
                                        );
                                        return Type::Unknown;
                                    }
                                }
                            }

                            if cls_name == "Task"
                                && (member == "await" || member == "join" || member == "wait")
                            {
                                if !args.is_empty() {
                                    self.error(
                                        format!("Task.{member}() expects 0 arguments"),
                                        *span,
                                    );
                                }
                                return Type::Unknown;
                            }

                            if let Some(cls) = self.symbols.classes.get(cls_name).cloned() {
                                if let Some(m) = cls.methods.get(member) {
                                    if cls_name == "ThreadPool" && member == "submit" {
                                        if args.is_empty() || args.len() > 2 {
                                            self.error(
                                                format!("Method 'submit' on class 'ThreadPool' expects 1 or 2 arguments, but got {}", args.len()),
                                                *span,
                                            );
                                        }
                                        for a in args {
                                            self.check_expression(a);
                                        }
                                        return m.return_type.clone();
                                    }
                                    if cls_name == "TcpStream" && member == "read" {
                                        if args.len() > 1 {
                                            self.error(
                                                format!("Method 'read' on class 'TcpStream' expects 0 or 1 argument, but got {}", args.len()),
                                                *span,
                                            );
                                        }
                                        for a in args {
                                            let a_ty = self.check_expression(a);
                                            if !a_ty.is_assignable_to(&Type::Int)
                                                && a_ty != Type::Unknown
                                            {
                                                self.error(format!("TcpStream.read max_len must be an int, found '{a_ty}'"), a.span());
                                            }
                                        }
                                        return Type::String;
                                    }
                                    if cls_name == "Arena" && member == "alloc" {
                                        if args.is_empty() {
                                            self.error("Method 'alloc' on class 'Arena' expects 1 argument", *span);
                                            return Type::Unknown;
                                        }
                                        let arg_ty = self.check_expression(&args[0]);
                                        return arg_ty;
                                    }
                                    if cls_name == "Arena" && member == "create" {
                                        for a in args {
                                            self.check_expression(a);
                                        }
                                        return Type::Class("Arena".to_string());
                                    }
                                    if cls_name == "Route" {
                                        if member == "with" {
                                            if args.is_empty() {
                                                self.error("Route.with requires at least 1 middleware argument", *span);
                                            }
                                            for a in args {
                                                self.check_expression(a);
                                            }
                                            return Type::Class("Route".to_string());
                                        }
                                        if member == "response" {
                                            if args.is_empty() || args.len() > 3 {
                                                self.error(format!("Route.response expects 1, 2, or 3 arguments, got {}", args.len()), *span);
                                            }
                                            for a in args {
                                                self.check_expression(a);
                                            }
                                            return Type::Class("Route".to_string());
                                        }
                                        if member == "request" || member == "body" {
                                            if args.len() != 1 {
                                                self.error(
                                                    format!("Route.{member} expects 1 argument"),
                                                    *span,
                                                );
                                            } else {
                                                self.check_expression(&args[0]);
                                            }
                                            return Type::Class("Route".to_string());
                                        }
                                        if member == "tag"
                                            || member == "summary"
                                            || member == "description"
                                        {
                                            if args.len() != 1 {
                                                self.error(
                                                    format!("Route.{member} expects 1 argument"),
                                                    *span,
                                                );
                                            } else {
                                                let a_ty = self.check_expression(&args[0]);
                                                if !a_ty.is_assignable_to(&Type::String)
                                                    && a_ty != Type::Unknown
                                                {
                                                    self.error(
                                                        format!(
                                                            "Route.{member} expects a string argument"
                                                        ),
                                                        args[0].span(),
                                                    );
                                                }
                                            }
                                            return Type::Class("Route".to_string());
                                        }
                                    }
                                    if m.params.len() != args.len() {
                                        self.error(
                                            format!("Method '{}' on class '{}' expects {} arguments, but got {}", member, cls_name, m.params.len(), args.len()),
                                            *span,
                                        );
                                    } else {
                                        for (arg_expr, (_, param_ty)) in args.iter().zip(&m.params)
                                        {
                                            let arg_ty = self.check_expression(arg_expr);
                                            if !arg_ty.is_assignable_to(param_ty) {
                                                self.error(
                                                    format!("Argument type '{arg_ty}' does not match method parameter type '{param_ty}'"),
                                                    arg_expr.span(),
                                                );
                                            }
                                        }
                                    }
                                    return m.return_type.clone();
                                } else if let Some(field) = cls.fields.get(member) {
                                    for a in args {
                                        self.check_expression(a);
                                    }
                                    match &field.ty {
                                        Type::Function { return_type, .. } => {
                                            return *return_type.clone()
                                        }
                                        _ => return Type::Unknown,
                                    }
                                } else {
                                    self.error(
                                        format!(
                                            "No method named '{member}' found on class '{cls_name}'"
                                        ),
                                        *m_span,
                                    );
                                    return Type::Unknown;
                                }
                            }
                        }
                        self.error(
                            format!("Cannot invoke method '{member}' on non-class type '{obj_ty}'"),
                            *span,
                        );
                        Type::Unknown
                    }
                    _ => {
                        self.error("Invalid call expression", *span);
                        Type::Unknown
                    }
                }
            }
            Expr::MemberAccess {
                object,
                member,
                span,
            } => {
                let obj_ty = self.check_expression(object);
                if (member == "length" || member == "len")
                    && matches!(obj_ty, Type::Array(_) | Type::String)
                {
                    return Type::Int;
                }
                if let Type::Class(cls_name) = &obj_ty {
                    if cls_name == "Map" && (member == "len" || member == "size") {
                        return Type::Int;
                    }
                    if let Some(cls) = self.symbols.classes.get(cls_name) {
                        if let Some(field) = cls.fields.get(member) {
                            return field.ty.clone();
                        } else {
                            self.error(
                                format!("Field '{member}' not found on class '{cls_name}'"),
                                *span,
                            );
                            return Type::Unknown;
                        }
                    }
                }
                if let Type::GenericInstance(cls_name, gen_args) = &obj_ty {
                    if cls_name == "Map" && (member == "len" || member == "size") {
                        return Type::Int;
                    }
                    if let Some(cls) = self.symbols.classes.get(cls_name) {
                        if let Some(field) = cls.fields.get(member) {
                            let mut mapping = HashMap::new();
                            for (p, a) in cls.generic_params.iter().zip(gen_args.iter()) {
                                mapping.insert(p.clone(), a.clone());
                            }
                            return substitute_type_mapping(&field.ty, &mapping);
                        } else {
                            self.error(
                                format!("Field '{member}' not found on class '{cls_name}'"),
                                *span,
                            );
                            return Type::Unknown;
                        }
                    }
                }
                self.error(
                    format!("Cannot access property '{member}' on non-class type '{obj_ty}'"),
                    *span,
                );
                Type::Unknown
            }
            Expr::New {
                class_name,
                generic_args,
                args,
                span,
            } => {
                let gen_types: Vec<Type> = generic_args.iter().map(Type::from_ast).collect();
                if class_name == "Map" {
                    if !gen_types.is_empty() {
                        return Type::GenericInstance("Map".to_string(), gen_types);
                    } else {
                        return Type::GenericInstance(
                            "Map".to_string(),
                            vec![Type::String, Type::Unknown],
                        );
                    }
                }
                if class_name == "Channel" {
                    if args.len() > 1 {
                        self.error(
                            format!(
                                "Constructor for 'Channel' expects 0 or 1 argument, but got {}",
                                args.len()
                            ),
                            *span,
                        );
                    } else if !args.is_empty() {
                        let arg_ty = self.check_expression(&args[0]);
                        if !arg_ty.is_assignable_to(&Type::Int) {
                            self.error(
                                format!(
                                    "Constructor argument for 'Channel' must be an int, found '{arg_ty}'"
                                ),
                                args[0].span(),
                            );
                        }
                    }
                    if !gen_types.is_empty() {
                        return Type::GenericInstance("Channel".to_string(), gen_types);
                    } else {
                        return Type::Class("Channel".to_string());
                    }
                }
                if class_name == "ThreadPool" {
                    if args.len() > 1 {
                        self.error(
                            format!(
                                "Constructor for 'ThreadPool' expects 0 or 1 argument, but got {}",
                                args.len()
                            ),
                            *span,
                        );
                    } else if !args.is_empty() {
                        let arg_ty = self.check_expression(&args[0]);
                        if !arg_ty.is_assignable_to(&Type::Int) {
                            self.error(format!("Constructor argument for 'ThreadPool' must be an int, found '{arg_ty}'"), args[0].span());
                        }
                    }
                    return Type::Class("ThreadPool".to_string());
                }
                if class_name == "Arena" {
                    if args.len() > 1 {
                        self.error(
                            format!(
                                "Constructor for 'Arena' expects 0 or 1 argument, but got {}",
                                args.len()
                            ),
                            *span,
                        );
                    } else if !args.is_empty() {
                        let arg_ty = self.check_expression(&args[0]);
                        if !arg_ty.is_assignable_to(&Type::Int) {
                            self.error(format!("Constructor argument for 'Arena' capacity must be an int, found '{arg_ty}'"), args[0].span());
                        }
                    }
                    return Type::Class("Arena".to_string());
                }
                if let Some(cls) = self.symbols.classes.get(class_name).cloned() {
                    let mut mapping = HashMap::new();
                    for (p, a) in cls.generic_params.iter().zip(gen_types.iter()) {
                        mapping.insert(p.clone(), a.clone());
                    }
                    if let Some(ctor) = &cls.constructor {
                        if ctor.params.len() != args.len() {
                            self.error(
                                format!(
                                    "Constructor for '{}' expects {} arguments, but got {}",
                                    class_name,
                                    ctor.params.len(),
                                    args.len()
                                ),
                                *span,
                            );
                        } else {
                            for (arg_expr, (_, param_ty)) in args.iter().zip(&ctor.params) {
                                let arg_ty = self.check_expression(arg_expr);
                                let exp_ty = substitute_type_mapping(param_ty, &mapping);
                                if !arg_ty.is_assignable_to(&exp_ty) {
                                    self.error(
                                        format!("Constructor argument type '{arg_ty}' does not match parameter type '{exp_ty}'"),
                                        arg_expr.span(),
                                    );
                                }
                            }
                        }
                    } else if !args.is_empty() {
                        self.error(
                            format!(
                                "Class '{}' has no constructor taking {} arguments",
                                class_name,
                                args.len()
                            ),
                            *span,
                        );
                    }
                    if !gen_types.is_empty() {
                        Type::GenericInstance(class_name.clone(), gen_types)
                    } else {
                        Type::Class(class_name.clone())
                    }
                } else {
                    self.error(
                        format!("Cannot instantiate undefined class '{class_name}'"),
                        *span,
                    );
                    Type::Unknown
                }
            }
            Expr::ArrayLit { elements, span: _ } => {
                if elements.is_empty() {
                    Type::Array(Box::new(Type::Unknown))
                } else {
                    let first_ty = self.check_expression(&elements[0]);
                    for elem in &elements[1..] {
                        let ty = self.check_expression(elem);
                        if !ty.is_assignable_to(&first_ty) {
                            self.error(
                                format!(
                                    "Array element type '{ty}' does not match expected '{first_ty}'"
                                ),
                                elem.span(),
                            );
                        }
                    }
                    Type::Array(Box::new(first_ty))
                }
            }
            Expr::Index { array, index, span } => {
                let arr_ty = self.check_expression(array);
                let idx_ty = self.check_expression(index);
                if let Type::GenericInstance(ref name, ref args) = arr_ty {
                    if name == "Map" {
                        let exp_k = args.first().unwrap_or(&Type::String);
                        if !idx_ty.is_assignable_to(exp_k) && idx_ty != Type::Unknown {
                            self.error(
                                format!("Map index key must be '{exp_k}', found '{idx_ty}'"),
                                index.span(),
                            );
                        }
                        return args.get(1).cloned().unwrap_or(Type::Unknown);
                    }
                }
                if let Type::Class(ref name) = arr_ty {
                    if name == "Map" {
                        return Type::Unknown;
                    }
                }
                if idx_ty != Type::Int && idx_ty != Type::Unknown {
                    self.error(
                        format!("Array index must be 'int', found '{idx_ty}'"),
                        index.span(),
                    );
                }
                match arr_ty {
                    Type::Array(elem_ty) => *elem_ty,
                    Type::String => Type::String,
                    Type::Unknown => Type::Unknown,
                    _ => {
                        self.error(format!("Cannot index non-array type '{arr_ty}'"), *span);
                        Type::Unknown
                    }
                }
            }
            Expr::Lambda {
                params,
                body_expr,
                body_block,
                span: _,
            } => {
                self.symbols.enter_scope();
                let mut param_types = Vec::new();
                for p in params {
                    let ty = Type::from_ast(&p.ty);
                    let _ = self.symbols.define_var(crate::symbols::VariableSymbol {
                        name: p.name.clone(),
                        ty: ty.clone(),
                        is_const: false,
                        span: p.span,
                    });
                    param_types.push(ty);
                }
                let ret_type = if let Some(body) = body_expr {
                    self.check_expression(body)
                } else if let Some(block) = body_block {
                    let prev_ret = self.current_return_type.take();
                    self.current_return_type = Some(Type::Unknown);
                    self.check_block(block);
                    self.current_return_type = prev_ret;
                    Type::Unknown
                } else {
                    Type::Void
                };
                self.symbols.exit_scope();
                Type::Function {
                    params: param_types,
                    return_type: Box::new(ret_type),
                }
            }
            Expr::Await { expr, .. } => {
                let inner_ty = self.check_expression(expr);
                match inner_ty {
                    Type::GenericInstance(ref name, ref args)
                        if name == "Task" && !args.is_empty() =>
                    {
                        args[0].clone()
                    }
                    Type::Class(ref name) if name == "Task" => Type::Unknown,
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            format!("'await' expression expects a Task, but found '{other}'"),
                            expr.span(),
                        );
                        Type::Unknown
                    }
                }
            }
        }
    }
}

fn substitute_type_mapping(ty: &Type, mapping: &HashMap<String, Type>) -> Type {
    match ty {
        Type::GenericParam(name) => mapping.get(name).cloned().unwrap_or_else(|| ty.clone()),
        Type::Array(elem) => Type::Array(Box::new(substitute_type_mapping(elem, mapping))),
        Type::GenericInstance(name, args) => {
            let new_args = args
                .iter()
                .map(|a| substitute_type_mapping(a, mapping))
                .collect();
            Type::GenericInstance(name.clone(), new_args)
        }
        Type::Function {
            params,
            return_type,
        } => {
            let new_params = params
                .iter()
                .map(|p| substitute_type_mapping(p, mapping))
                .collect();
            let new_ret = Box::new(substitute_type_mapping(return_type, mapping));
            Type::Function {
                params: new_params,
                return_type: new_ret,
            }
        }
        other => other.clone(),
    }
}
