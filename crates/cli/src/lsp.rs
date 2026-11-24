use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object().and_then(|m| m.get(key))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn to_json_string(&self) -> String {
        match self {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && !n.is_infinite() && !n.is_nan() {
                    format!("{n:.0}")
                } else {
                    n.to_string()
                }
            }
            JsonValue::String(s) => {
                let mut out = String::from("\"");
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        other => out.push(other),
                    }
                }
                out.push('"');
                out
            }
            JsonValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", items.join(","))
            }
            JsonValue::Object(map) => {
                let mut pairs: Vec<String> = map
                    .iter()
                    .map(|(k, v)| {
                        let mut k_esc = String::from("\"");
                        for c in k.chars() {
                            if c == '"' {
                                k_esc.push_str("\\\"");
                            } else {
                                k_esc.push(c);
                            }
                        }
                        k_esc.push('"');
                        format!("{}:{}", k_esc, v.to_json_string())
                    })
                    .collect();
                pairs.sort();
                format!("{{{}}}", pairs.join(","))
            }
        }
    }
}

pub struct JsonParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> JsonParser<'a> {
    pub fn parse(input: &'a str) -> Result<JsonValue, String> {
        let mut p = Self {
            chars: input.chars().peekable(),
        };
        let val = p.parse_value()?;
        Ok(val)
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.chars.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string(),
            Some('t') | Some('f') => self.parse_bool(),
            Some('n') => self.parse_null(),
            Some(&c) if c.is_ascii_digit() || c == '-' => self.parse_number(),
            Some(&other) => Err(format!("Unexpected character '{other}'")),
            None => Ok(JsonValue::Null),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.chars.next(); // consume '{'
        let mut map = HashMap::new();
        loop {
            self.skip_whitespace();
            if let Some(&'}') = self.chars.peek() {
                self.chars.next();
                break;
            }
            let key = match self.parse_value()? {
                JsonValue::String(s) => s,
                _ => return Err("Expected string key in object".to_string()),
            };
            self.skip_whitespace();
            if let Some(&':') = self.chars.peek() {
                self.chars.next();
            } else {
                return Err("Expected ':' after object key".to_string());
            }
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_whitespace();
            if let Some(&',') = self.chars.peek() {
                self.chars.next();
            } else if let Some(&'}') = self.chars.peek() {
                self.chars.next();
                break;
            } else {
                break;
            }
        }
        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.chars.next(); // consume '['
        let mut arr = Vec::new();
        loop {
            self.skip_whitespace();
            if let Some(&']') = self.chars.peek() {
                self.chars.next();
                break;
            }
            let val = self.parse_value()?;
            arr.push(val);
            self.skip_whitespace();
            if let Some(&',') = self.chars.peek() {
                self.chars.next();
            } else if let Some(&']') = self.chars.peek() {
                self.chars.next();
                break;
            } else {
                break;
            }
        }
        Ok(JsonValue::Array(arr))
    }

    fn parse_string(&mut self) -> Result<JsonValue, String> {
        self.chars.next(); // consume '"'
        let mut s = String::new();
        while let Some(c) = self.chars.next() {
            if c == '"' {
                break;
            } else if c == '\\' {
                if let Some(esc) = self.chars.next() {
                    match esc {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        '/' => s.push('/'),
                        'b' => s.push('\x08'),
                        'f' => s.push('\x0c'),
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        'u' => {
                            let mut hex = String::new();
                            for _ in 0..4 {
                                if let Some(h) = self.chars.next() {
                                    hex.push(h);
                                }
                            }
                            if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                if let Some(ch) = char::from_u32(code) {
                                    s.push(ch);
                                }
                            }
                        }
                        other => s.push(other),
                    }
                }
            } else {
                s.push(c);
            }
        }
        Ok(JsonValue::String(s))
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        let num: f64 = s.parse().unwrap_or(0.0);
        Ok(JsonValue::Number(num))
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.chars.peek() == Some(&'t') {
            for _ in 0..4 {
                self.chars.next();
            }
            Ok(JsonValue::Bool(true))
        } else {
            for _ in 0..5 {
                self.chars.next();
            }
            Ok(JsonValue::Bool(false))
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        for _ in 0..4 {
            self.chars.next();
        }
        Ok(JsonValue::Null)
    }
}

pub fn offset_to_position(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0;
    let mut col = 0;
    let clamped_offset = offset.min(source.len());

    for (idx, ch) in source.char_indices() {
        if idx >= clamped_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

pub fn position_to_offset(source: &str, line: u32, col: u32) -> usize {
    let mut cur_line = 0;
    let mut cur_col = 0;

    for (idx, ch) in source.char_indices() {
        if cur_line == line && cur_col == col {
            return idx;
        }
        if ch == '\n' {
            if cur_line == line {
                return idx;
            }
            cur_line += 1;
            cur_col = 0;
        } else {
            cur_col += 1;
        }
    }

    source.len()
}

pub struct LspServer {
    documents: HashMap<String, String>,
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let stdout = io::stdout();
        let mut writer = stdout.lock();

        loop {
            // Read headers (Content-Length: <len>\r\n\r\n)
            let mut content_length: Option<usize> = None;
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => return, // EOF
                    Ok(_) => {}
                    Err(_) => return,
                }

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }

                let lower = trimmed.to_ascii_lowercase();
                if let Some(rest) = lower.strip_prefix("content-length:") {
                    if let Ok(len) = rest.trim().parse::<usize>() {
                        content_length = Some(len);
                    }
                }
            }

            let len = match content_length {
                Some(l) => l,
                None => continue,
            };

            let mut body = vec![0u8; len];
            if reader.read_exact(&mut body).is_err() {
                return;
            }

            let body_str = match String::from_utf8(body) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let request = match JsonParser::parse(&body_str) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(resp) = self.handle_message(&request) {
                let resp_str = resp.to_json_string();
                let header = format!("Content-Length: {}\r\n\r\n", resp_str.len());
                let _ = writer.write_all(header.as_bytes());
                let _ = writer.write_all(resp_str.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    pub fn handle_message(&mut self, msg: &JsonValue) -> Option<JsonValue> {
        let method = msg.get("method").and_then(|m| m.as_str())?;
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(JsonValue::Null);

        match method {
            "initialize" => {
                let mut capabilities = HashMap::new();
                capabilities.insert("textDocumentSync".to_string(), JsonValue::Number(1.0));
                capabilities.insert("hoverProvider".to_string(), JsonValue::Bool(true));
                capabilities.insert("definitionProvider".to_string(), JsonValue::Bool(true));
                capabilities.insert("documentSymbolProvider".to_string(), JsonValue::Bool(true));

                let mut completion = HashMap::new();
                completion.insert(
                    "triggerCharacters".to_string(),
                    JsonValue::Array(vec![
                        JsonValue::String(".".to_string()),
                        JsonValue::String(":".to_string()),
                    ]),
                );
                capabilities.insert(
                    "completionProvider".to_string(),
                    JsonValue::Object(completion),
                );

                let mut server_info = HashMap::new();
                server_info.insert(
                    "name".to_string(),
                    JsonValue::String("syntra-lsp".to_string()),
                );
                server_info.insert(
                    "version".to_string(),
                    JsonValue::String("0.1.0".to_string()),
                );

                let mut result = HashMap::new();
                result.insert("capabilities".to_string(), JsonValue::Object(capabilities));
                result.insert("serverInfo".to_string(), JsonValue::Object(server_info));

                let mut resp = HashMap::new();
                resp.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
                if let Some(req_id) = id {
                    resp.insert("id".to_string(), req_id);
                }
                resp.insert("result".to_string(), JsonValue::Object(result));
                Some(JsonValue::Object(resp))
            }
            "initialized" => None,
            "textDocument/didOpen" => {
                if let Some(doc) = params.get("textDocument") {
                    if let (Some(uri), Some(text)) = (
                        doc.get("uri").and_then(|u| u.as_str()),
                        doc.get("text").and_then(|t| t.as_str()),
                    ) {
                        self.documents.insert(uri.to_string(), text.to_string());
                        return self.publish_diagnostics(uri, text);
                    }
                }
                None
            }
            "textDocument/didChange" => {
                if let (Some(doc), Some(changes)) = (
                    params.get("textDocument"),
                    params.get("contentChanges").and_then(|c| c.as_array()),
                ) {
                    if let (Some(uri), Some(first_change)) =
                        (doc.get("uri").and_then(|u| u.as_str()), changes.first())
                    {
                        if let Some(text) = first_change.get("text").and_then(|t| t.as_str()) {
                            self.documents.insert(uri.to_string(), text.to_string());
                            return self.publish_diagnostics(uri, text);
                        }
                    }
                }
                None
            }
            "textDocument/didClose" => {
                if let Some(doc) = params.get("textDocument") {
                    if let Some(uri) = doc.get("uri").and_then(|u| u.as_str()) {
                        self.documents.remove(uri);
                        return self.clear_diagnostics(uri);
                    }
                }
                None
            }
            "textDocument/hover" => {
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(|u| u.as_str())?;
                let pos = params.get("position")?;
                let line = pos.get("line").and_then(|l| l.as_i64())? as u32;
                let col = pos.get("character").and_then(|c| c.as_i64())? as u32;

                let text = self.documents.get(uri)?;
                let hover_info = self.compute_hover(text, line, col);

                let mut resp = HashMap::new();
                resp.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
                if let Some(req_id) = id {
                    resp.insert("id".to_string(), req_id);
                }

                if let Some(info) = hover_info {
                    let mut contents = HashMap::new();
                    contents.insert(
                        "kind".to_string(),
                        JsonValue::String("markdown".to_string()),
                    );
                    contents.insert("value".to_string(), JsonValue::String(info));

                    let mut res = HashMap::new();
                    res.insert("contents".to_string(), JsonValue::Object(contents));
                    resp.insert("result".to_string(), JsonValue::Object(res));
                } else {
                    resp.insert("result".to_string(), JsonValue::Null);
                }

                Some(JsonValue::Object(resp))
            }
            "textDocument/completion" => {
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(|u| u.as_str());
                let pos = params.get("position");
                let line = pos
                    .and_then(|p| p.get("line"))
                    .and_then(|l| l.as_i64())
                    .map(|l| l as u32);
                let col = pos
                    .and_then(|p| p.get("character"))
                    .and_then(|c| c.as_i64())
                    .map(|c| c as u32);

                let completions = self.compute_completions(uri, line, col);

                let mut resp = HashMap::new();
                resp.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
                if let Some(req_id) = id {
                    resp.insert("id".to_string(), req_id);
                }
                let mut comp_list = HashMap::new();
                comp_list.insert("isIncomplete".to_string(), JsonValue::Bool(false));
                comp_list.insert("items".to_string(), JsonValue::Array(completions.clone()));
                resp.insert("result".to_string(), JsonValue::Object(comp_list));
                Some(JsonValue::Object(resp))
            }
            "textDocument/definition" => {
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(|u| u.as_str())?;
                let pos = params.get("position")?;
                let line = pos.get("line").and_then(|l| l.as_i64())? as u32;
                let col = pos.get("character").and_then(|c| c.as_i64())? as u32;

                let text = self.documents.get(uri)?;
                let def_location = self.compute_definition(uri, text, line, col);

                let mut resp = HashMap::new();
                resp.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
                if let Some(req_id) = id {
                    resp.insert("id".to_string(), req_id);
                }
                resp.insert(
                    "result".to_string(),
                    def_location.unwrap_or(JsonValue::Null),
                );
                Some(JsonValue::Object(resp))
            }
            "textDocument/documentSymbol" => {
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(|u| u.as_str())?;
                let text = self.documents.get(uri)?;
                let symbols = self.compute_document_symbols(text);

                let mut resp = HashMap::new();
                resp.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
                if let Some(req_id) = id {
                    resp.insert("id".to_string(), req_id);
                }
                resp.insert("result".to_string(), JsonValue::Array(symbols));
                Some(JsonValue::Object(resp))
            }
            "shutdown" => {
                let mut resp = HashMap::new();
                resp.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
                if let Some(req_id) = id {
                    resp.insert("id".to_string(), req_id);
                }
                resp.insert("result".to_string(), JsonValue::Null);
                Some(JsonValue::Object(resp))
            }
            "exit" => {
                std::process::exit(0);
            }
            _ => {
                if let Some(req_id) = id {
                    let mut resp = HashMap::new();
                    resp.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
                    resp.insert("id".to_string(), req_id);
                    resp.insert("result".to_string(), JsonValue::Null);
                    Some(JsonValue::Object(resp))
                } else {
                    None
                }
            }
        }
    }

    fn publish_diagnostics(&self, uri: &str, text: &str) -> Option<JsonValue> {
        let mut diag_items = Vec::new();

        match syntra_syntax::parse(text) {
            Ok(program) => {
                if let Err(type_diags) = syntra_frontend::check(&program) {
                    for d in type_diags {
                        diag_items.push(self.create_diagnostic(
                            text,
                            &d.message,
                            d.span.start,
                            d.span.end,
                            1,
                        ));
                    }
                }
            }
            Err(parse_diags) => {
                for d in parse_diags {
                    diag_items.push(self.create_diagnostic(
                        text,
                        &d.message,
                        d.span.start,
                        d.span.end,
                        1,
                    ));
                }
            }
        }

        let mut params = HashMap::new();
        params.insert("uri".to_string(), JsonValue::String(uri.to_string()));
        params.insert("diagnostics".to_string(), JsonValue::Array(diag_items));

        let mut notif = HashMap::new();
        notif.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
        notif.insert(
            "method".to_string(),
            JsonValue::String("textDocument/publishDiagnostics".to_string()),
        );
        notif.insert("params".to_string(), JsonValue::Object(params));

        Some(JsonValue::Object(notif))
    }

    fn clear_diagnostics(&self, uri: &str) -> Option<JsonValue> {
        let mut params = HashMap::new();
        params.insert("uri".to_string(), JsonValue::String(uri.to_string()));
        params.insert("diagnostics".to_string(), JsonValue::Array(Vec::new()));

        let mut notif = HashMap::new();
        notif.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
        notif.insert(
            "method".to_string(),
            JsonValue::String("textDocument/publishDiagnostics".to_string()),
        );
        notif.insert("params".to_string(), JsonValue::Object(params));

        Some(JsonValue::Object(notif))
    }

    fn create_diagnostic(
        &self,
        text: &str,
        msg: &str,
        start_byte: usize,
        end_byte: usize,
        severity: i64,
    ) -> JsonValue {
        let (s_line, s_col) = offset_to_position(text, start_byte);
        let (e_line, e_col) = offset_to_position(text, end_byte.max(start_byte + 1));

        let mut start_pos = HashMap::new();
        start_pos.insert("line".to_string(), JsonValue::Number(s_line as f64));
        start_pos.insert("character".to_string(), JsonValue::Number(s_col as f64));

        let mut end_pos = HashMap::new();
        end_pos.insert("line".to_string(), JsonValue::Number(e_line as f64));
        end_pos.insert("character".to_string(), JsonValue::Number(e_col as f64));

        let mut range = HashMap::new();
        range.insert("start".to_string(), JsonValue::Object(start_pos));
        range.insert("end".to_string(), JsonValue::Object(end_pos));

        let mut diag = HashMap::new();
        diag.insert("range".to_string(), JsonValue::Object(range));
        diag.insert("severity".to_string(), JsonValue::Number(severity as f64));
        diag.insert("message".to_string(), JsonValue::String(msg.to_string()));
        diag.insert(
            "source".to_string(),
            JsonValue::String("syntra".to_string()),
        );

        JsonValue::Object(diag)
    }

    fn compute_hover(&self, text: &str, line: u32, col: u32) -> Option<String> {
        let offset = position_to_offset(text, line, col);
        let word = extract_word_at(text, offset)?;

        // 1. Built-in keywords
        match word {
            "Task" => return Some("```syntra\nclass Task\n```\nSyntra Task concurrency primitive (`Task.spawn`, `Task.sleep`, `join()`).".to_string()),
            "Channel" => return Some("```syntra\nclass Channel<T>\n```\nSyntra thread-safe synchronization channel (`send`, `recv`, `trySend`, `tryRecv`, `Channel.select`, `close`, `len`).".to_string()),
            "ThreadPool" => return Some("```syntra\nclass ThreadPool\n```\nSyntra thread pool task executor (`new ThreadPool(workers)`, `submit(fn, arg)`, `shutdown()`, `workers()`).".to_string()),
            "Map" => return Some("```syntra\nclass Map<K, V>\n```\nSyntra high-performance hash map collection (`set`, `get`, `has`, `remove`, `len`, `keys`, `values`, `clear`).".to_string()),
            "File" => return Some("```syntra\nclass File\n```\nSyntra File I/O subsystem (`readText`, `writeText`, `appendText`, `exists`, `delete`, `size`, `readLines`).".to_string()),
            "System" => return Some("```syntra\nnamespace System\n```\nSyntra operating system interface (`args`, `exit`, `getEnv`, `timeMs`).".to_string()),
            "Math" => return Some("```syntra\nnamespace Math\n```\nSyntra standard mathematical utilities (`min`, `max`, `abs`, `clamp`, `pow`, `sqrt`).".to_string()),
            "Json" => return Some("```syntra\nnamespace Json\n```\nSyntra JSON serialization engine (`Json.parse`, `Json.stringify`).".to_string()),
            "int" | "float" | "bool" | "string" | "void" => {
                return Some(format!("```syntra\nprimitive type {word}\n```"));
            }
            "fn" | "func" => return Some("```syntra\nfn name(Type arg): ReturnType\n```\nDeclares a top-level or member function.".to_string()),
            "class" => return Some("```syntra\nclass Name { ... }\n```\nDeclares an object-oriented class.".to_string()),
            _ => {}
        }

        // 2. Program symbols from AST
        if let Ok(prog) = syntra_syntax::parse(text) {
            for item in &prog.items {
                match item {
                    syntra_syntax::ast::Item::Function(f) => {
                        if f.name == word {
                            let params_str = f
                                .params
                                .iter()
                                .map(|p| format!("{} {}", p.ty.name, p.name))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let ret_str = f
                                .return_type
                                .as_ref()
                                .map(|r| r.name.clone())
                                .unwrap_or_else(|| "void".to_string());
                            let doc_str = f
                                .doc
                                .as_ref()
                                .map(|d| format!("\n\n{d}"))
                                .unwrap_or_default();
                            return Some(format!(
                                "```syntra\nfn {}({}): {}{}\n```",
                                f.name, params_str, ret_str, doc_str
                            ));
                        }
                    }
                    syntra_syntax::ast::Item::Class(cls) => {
                        if cls.name == word {
                            let doc_str = cls
                                .doc
                                .as_ref()
                                .map(|d| format!("\n\n{d}"))
                                .unwrap_or_default();
                            return Some(format!("```syntra\nclass {}{}\n```", cls.name, doc_str));
                        }
                        for m in &cls.members {
                            if let syntra_syntax::ast::ClassMember::Method(method) = m {
                                if method.func.name == word {
                                    let params_str = method
                                        .func
                                        .params
                                        .iter()
                                        .map(|p| format!("{} {}", p.ty.name, p.name))
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    let ret_str = method
                                        .func
                                        .return_type
                                        .as_ref()
                                        .map(|r| r.name.clone())
                                        .unwrap_or_else(|| "void".to_string());
                                    let prefix = if method.is_static { "static fn" } else { "fn" };
                                    return Some(format!(
                                        "```syntra\n{}.{}({}): {}\n```",
                                        cls.name, prefix, params_str, ret_str
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    fn make_completion_item(label: &str, detail: &str, kind: i64) -> JsonValue {
        let mut item = HashMap::new();
        item.insert("label".to_string(), JsonValue::String(label.to_string()));
        item.insert("detail".to_string(), JsonValue::String(detail.to_string()));
        item.insert("kind".to_string(), JsonValue::Number(kind as f64));
        JsonValue::Object(item)
    }

    fn compute_completions(
        &self,
        uri: Option<&str>,
        line: Option<u32>,
        col: Option<u32>,
    ) -> Vec<JsonValue> {
        let mut results = Vec::new();

        let mut after_dot = false;
        let mut prev_word: Option<String> = None;

        if let (Some(u), Some(l), Some(c)) = (uri, line, col) {
            if let Some(text) = self.documents.get(u) {
                let offset = position_to_offset(text, l, c);
                if offset > 0 {
                    let before = &text[..offset];
                    let trimmed = before.trim_end();
                    if let Some(stripped) = trimmed.strip_suffix('.') {
                        after_dot = true;
                        let without_dot = stripped.trim_end();
                        let last_word_start = without_dot
                            .rfind(|ch: char| !ch.is_alphanumeric() && ch != '_')
                            .map(|idx| idx + 1)
                            .unwrap_or(0);
                        let w = without_dot[last_word_start..].to_string();
                        if !w.is_empty() {
                            prev_word = Some(w);
                        }
                    }
                }
            }
        }

        if after_dot {
            match prev_word.as_deref() {
                Some("System") => {
                    let items = [
                        ("println", "System.println(msg) - Print with newline", 2),
                        ("print", "System.print(msg) - Print without newline", 2),
                        ("args", "System.args() - Command line arguments", 2),
                        ("exit", "System.exit(code) - Terminate process", 2),
                        (
                            "getEnv",
                            "System.getEnv(key) - Read environment variable",
                            2,
                        ),
                        ("timeMs", "System.timeMs() - Current timestamp in ms", 2),
                    ];
                    for (l, d, k) in items {
                        results.push(Self::make_completion_item(l, d, k));
                    }
                    return results;
                }
                Some("Task") => {
                    let items = [
                        ("spawn", "Task.spawn(fn) - Spawn asynchronous task", 2),
                        ("sleep", "Task.sleep(ms) - Sleep current task", 2),
                    ];
                    for (l, d, k) in items {
                        results.push(Self::make_completion_item(l, d, k));
                    }
                    return results;
                }
                Some("Channel") => {
                    let items = [(
                        "select",
                        "Channel.select(channels) - Channel multiplexer",
                        2,
                    )];
                    for (l, d, k) in items {
                        results.push(Self::make_completion_item(l, d, k));
                    }
                    return results;
                }
                Some("Math") => {
                    let items = [
                        ("min", "Math.min(a, b)", 2),
                        ("max", "Math.max(a, b)", 2),
                        ("abs", "Math.abs(x)", 2),
                        ("clamp", "Math.clamp(x, min, max)", 2),
                        ("pow", "Math.pow(base, exp)", 2),
                        ("sqrt", "Math.sqrt(x)", 2),
                    ];
                    for (l, d, k) in items {
                        results.push(Self::make_completion_item(l, d, k));
                    }
                    return results;
                }
                Some("File") => {
                    let items = [
                        ("readText", "File.readText(path) - Read file as string", 2),
                        (
                            "writeText",
                            "File.writeText(path, text) - Write string to file",
                            2,
                        ),
                        (
                            "appendText",
                            "File.appendText(path, text) - Append string to file",
                            2,
                        ),
                        ("exists", "File.exists(path) - Check file existence", 2),
                        ("delete", "File.delete(path) - Delete file", 2),
                        ("size", "File.size(path) - Get file size in bytes", 2),
                        (
                            "readLines",
                            "File.readLines(path) - Read file as array of lines",
                            2,
                        ),
                    ];
                    for (l, d, k) in items {
                        results.push(Self::make_completion_item(l, d, k));
                    }
                    return results;
                }
                Some("Json") => {
                    let items = [
                        ("parse", "Json.parse(jsonStr) - Parse JSON string", 2),
                        (
                            "stringify",
                            "Json.stringify(value) - Serialize to JSON string",
                            2,
                        ),
                    ];
                    for (l, d, k) in items {
                        results.push(Self::make_completion_item(l, d, k));
                    }
                    return results;
                }
                _ => {
                    let items = [
                        ("send", "ch.send(val) - Send value into channel", 2),
                        ("recv", "ch.recv() - Receive value from channel", 2),
                        (
                            "trySend",
                            "ch.trySend(val) - Non-blocking send into channel",
                            2,
                        ),
                        (
                            "tryRecv",
                            "ch.tryRecv() - Non-blocking receive from channel",
                            2,
                        ),
                        ("close", "ch.close() - Close channel", 2),
                        ("len", "collection.len() - Get length / count", 2),
                        ("submit", "pool.submit(fn) - Submit task to ThreadPool", 2),
                        (
                            "shutdown",
                            "pool.shutdown() - Gracefully shutdown ThreadPool",
                            2,
                        ),
                        ("await", "task.await() - Await task completion", 2),
                        ("isDone", "task.isDone() - Check task status", 2),
                        ("get", "map.get(key) - Get item from map", 2),
                        ("set", "map.set(key, val) - Set item in map", 2),
                        ("has", "map.has(key) - Check key existence", 2),
                        ("remove", "map.remove(key) - Remove key from map", 2),
                        ("keys", "map.keys() - Get all keys", 2),
                        ("values", "map.values() - Get all values", 2),
                        ("clear", "map.clear() - Clear all entries", 2),
                    ];
                    for (l, d, k) in items {
                        results.push(Self::make_completion_item(l, d, k));
                    }
                    return results;
                }
            }
        }

        let keywords = [
            ("fn", "Function declaration", 14),
            ("func", "Function declaration", 14),
            ("class", "Class declaration", 7),
            ("interface", "Interface declaration", 8),
            ("import", "Module import", 9),
            ("return", "Return statement", 14),
            ("if", "Conditional statement", 14),
            ("else", "Else branch", 14),
            ("for", "For loop", 14),
            ("in", "For-in iterator", 14),
            ("while", "While loop", 14),
            ("new", "Object instantiation", 14),
            ("this", "Current class instance", 14),
            ("public", "Public visibility", 14),
            ("private", "Private visibility", 14),
            ("static", "Static member modifier", 14),
            ("select", "Channel select", 14),
            ("case", "Select case", 14),
            ("default", "Select default", 14),
            ("try", "Try block", 14),
            ("catch", "Catch block", 14),
            ("finally", "Finally block", 14),
            ("throw", "Throw exception", 14),
            ("ThreadPool", "Thread pool task executor", 7),
            ("Task", "Concurrency task runner", 7),
            ("Channel", "Synchronization channel", 7),
            ("Map", "Key-value hash map", 7),
            ("File", "File system operations", 7),
            ("System", "System utility namespace", 9),
            ("Math", "Mathematical functions", 9),
            ("Json", "JSON serialization engine", 9),
            ("int", "64-bit integer type", 6),
            ("float", "64-bit float type", 6),
            ("bool", "Boolean type", 6),
            ("string", "UTF-8 string type", 6),
            ("void", "Void type", 6),
        ];

        for (l, d, k) in keywords {
            results.push(Self::make_completion_item(l, d, k));
        }

        if let Some(u) = uri {
            if let Some(text) = self.documents.get(u) {
                if let Ok(prog) = syntra_syntax::parse(text) {
                    for item in &prog.items {
                        match item {
                            syntra_syntax::ast::Item::Function(f) => {
                                results.push(Self::make_completion_item(
                                    &f.name,
                                    &format!("fn {}(...)", f.name),
                                    3,
                                ));
                            }
                            syntra_syntax::ast::Item::Class(c) => {
                                results.push(Self::make_completion_item(
                                    &c.name,
                                    &format!("class {}", c.name),
                                    7,
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        results
    }

    fn compute_definition(&self, uri: &str, text: &str, line: u32, col: u32) -> Option<JsonValue> {
        let offset = position_to_offset(text, line, col);
        let word = extract_word_at(text, offset)?;

        if let Ok(prog) = syntra_syntax::parse(text) {
            for item in &prog.items {
                match item {
                    syntra_syntax::ast::Item::Function(f) => {
                        if f.name == word {
                            return Some(self.create_location(uri, text, f.span.start, f.span.end));
                        }
                    }
                    syntra_syntax::ast::Item::Class(cls) => {
                        if cls.name == word {
                            return Some(self.create_location(
                                uri,
                                text,
                                cls.span.start,
                                cls.span.end,
                            ));
                        }
                        for m in &cls.members {
                            if let syntra_syntax::ast::ClassMember::Method(method) = m {
                                if method.func.name == word {
                                    return Some(self.create_location(
                                        uri,
                                        text,
                                        method.func.span.start,
                                        method.func.span.end,
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    fn compute_document_symbols(&self, text: &str) -> Vec<JsonValue> {
        let mut symbols = Vec::new();

        if let Ok(prog) = syntra_syntax::parse(text) {
            for item in &prog.items {
                match item {
                    syntra_syntax::ast::Item::Function(f) => {
                        symbols.push(self.create_symbol(
                            &f.name,
                            12,
                            text,
                            f.span.start,
                            f.span.end,
                        ));
                    }
                    syntra_syntax::ast::Item::Class(cls) => {
                        symbols.push(self.create_symbol(
                            &cls.name,
                            5,
                            text,
                            cls.span.start,
                            cls.span.end,
                        ));
                    }
                    syntra_syntax::ast::Item::Namespace(ns) => {
                        symbols.push(self.create_symbol(
                            &ns.name,
                            3,
                            text,
                            ns.span.start,
                            ns.span.end,
                        ));
                    }
                    _ => {}
                }
            }
        }

        symbols
    }

    fn create_location(
        &self,
        uri: &str,
        text: &str,
        start_byte: usize,
        end_byte: usize,
    ) -> JsonValue {
        let (s_line, s_col) = offset_to_position(text, start_byte);
        let (e_line, e_col) = offset_to_position(text, end_byte);

        let mut start_pos = HashMap::new();
        start_pos.insert("line".to_string(), JsonValue::Number(s_line as f64));
        start_pos.insert("character".to_string(), JsonValue::Number(s_col as f64));

        let mut end_pos = HashMap::new();
        end_pos.insert("line".to_string(), JsonValue::Number(e_line as f64));
        end_pos.insert("character".to_string(), JsonValue::Number(e_col as f64));

        let mut range = HashMap::new();
        range.insert("start".to_string(), JsonValue::Object(start_pos));
        range.insert("end".to_string(), JsonValue::Object(end_pos));

        let mut loc = HashMap::new();
        loc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
        loc.insert("range".to_string(), JsonValue::Object(range));

        JsonValue::Object(loc)
    }

    fn create_symbol(
        &self,
        name: &str,
        kind: i64,
        text: &str,
        start_byte: usize,
        end_byte: usize,
    ) -> JsonValue {
        let (s_line, s_col) = offset_to_position(text, start_byte);
        let (e_line, e_col) = offset_to_position(text, end_byte);

        let mut start_pos = HashMap::new();
        start_pos.insert("line".to_string(), JsonValue::Number(s_line as f64));
        start_pos.insert("character".to_string(), JsonValue::Number(s_col as f64));

        let mut end_pos = HashMap::new();
        end_pos.insert("line".to_string(), JsonValue::Number(e_line as f64));
        end_pos.insert("character".to_string(), JsonValue::Number(e_col as f64));

        let mut range = HashMap::new();
        range.insert("start".to_string(), JsonValue::Object(start_pos));
        range.insert("end".to_string(), JsonValue::Object(end_pos));

        let mut sym = HashMap::new();
        sym.insert("name".to_string(), JsonValue::String(name.to_string()));
        sym.insert("kind".to_string(), JsonValue::Number(kind as f64));
        sym.insert("range".to_string(), JsonValue::Object(range.clone()));
        sym.insert("selectionRange".to_string(), JsonValue::Object(range));

        JsonValue::Object(sym)
    }
}

fn extract_word_at(text: &str, offset: usize) -> Option<&str> {
    if text.is_empty() {
        return None;
    }
    let clamped = offset.min(text.len() - 1);

    let bytes = text.as_bytes();
    let mut start = clamped;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }

    let mut end = clamped;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    if start == end {
        None
    } else {
        Some(&text[start..end])
    }
}

pub fn run_server() {
    let mut server = LspServer::new();
    server.run();
}
