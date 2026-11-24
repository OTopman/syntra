use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")] // Skip whitespace
pub enum Token {
    // First-class Doc Comments: /// ...
    #[regex(r"///[^\r\n]*", |lex| {
        let s = lex.slice();
        let text = s.strip_prefix("///").unwrap_or(s);
        let text = text.strip_prefix(' ').unwrap_or(text);
        text.to_string()
    })]
    DocComment(String),

    // Regular line comments: // ... (filtered in tokenize)
    #[regex(r"//[^\r\n]*", |_| ())]
    Comment,

    // Keywords - Modules & Namespaces
    #[token("namespace")]
    Namespace,
    #[token("import")]
    Import,

    // Keywords - Functions & Control Flow
    #[token("func")]
    #[token("fn")]
    Func,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("return")]
    Return,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("try")]
    Try,
    #[token("catch")]
    Catch,
    #[token("finally")]
    Finally,
    #[token("throw")]
    Throw,

    // Keywords - OOP & Types
    #[token("class")]
    Class,
    #[token("custom")]
    Custom,
    #[token("struct")]
    Struct,
    #[token("interface")]
    Interface,
    #[token("implements")]
    Implements,
    #[token("extends")]
    Extends,
    #[token("new")]
    New,
    #[token("this")]
    This,
    #[token("super")]
    Super,
    #[token("public")]
    Public,
    #[token("private")]
    Private,
    #[token("static")]
    Static,
    #[token("override")]
    Override,
    #[token("const")]
    Const,

    // Core Built-in Type Keywords
    #[token("int")]
    IntType,
    #[token("float")]
    FloatType,
    #[token("bool")]
    BoolType,
    #[token("string")]
    StringType,
    #[token("void")]
    VoidType,

    // Boolean Literals
    #[token("true")]
    True,
    #[token("false")]
    False,

    // Punctuation & Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,

    // Operators - Arithmetic
    #[token("=")]
    Assign,
    #[token("=>")]
    FatArrow,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    // Operators - Bitwise
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,
    #[token("<<")]
    LessLess,
    #[token(">>")]
    GreaterGreater,

    // Operators - Comparisons
    #[token("==")]
    EqualEqual,
    #[token("!=")]
    NotEqual,
    #[token("<")]
    Less,
    #[token("<=")]
    LessEqual,
    #[token(">")]
    Greater,
    #[token(">=")]
    GreaterEqual,

    // Operators - Logical
    #[token("&&")]
    AmpAmp,
    #[token("||")]
    PipePipe,

    // Literals & Identifiers
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    FloatLit(f64),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntegerLit(i64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        let inner = &s[1..s.len()-1];
        let mut unescaped = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => unescaped.push('\n'),
                    Some('t') => unescaped.push('\t'),
                    Some('r') => unescaped.push('\r'),
                    Some('x') => {
                        let h1 = chars.next();
                        let h2 = chars.next();
                        if let (Some(c1), Some(c2)) = (h1, h2) {
                            if let Ok(b) = u8::from_str_radix(&format!("{c1}{c2}"), 16) {
                                unescaped.push(b as char);
                            } else {
                                unescaped.push('\\');
                                unescaped.push('x');
                                unescaped.push(c1);
                                unescaped.push(c2);
                            }
                        } else {
                            unescaped.push('\\');
                            unescaped.push('x');
                            if let Some(c1) = h1 { unescaped.push(c1); }
                        }
                    }
                    Some('\\') => unescaped.push('\\'),
                    Some('"') => unescaped.push('"'),
                    Some(other) => {
                        unescaped.push('\\');
                        unescaped.push(other);
                    }
                    None => unescaped.push('\\'),
                }
            } else {
                unescaped.push(c);
            }
        }
        Some(unescaped)
    })]
    StringLit(String),

    #[regex(r#"\$"([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(s[2..s.len()-1].to_string())
    })]
    InterpolatedStringLit(String),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
}
