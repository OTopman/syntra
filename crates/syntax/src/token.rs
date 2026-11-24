use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")] // Skip whitespace
#[logos(skip r"//[^\n]*")]       // Skip line comments
pub enum Token {
    // Keywords - Functions & Control Flow
    #[token("func")]
    #[token("fn")]
    Func,
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

    // Keywords - OOP & Types
    #[token("class")]
    Class,
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
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,

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

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
}
