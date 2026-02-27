/// All token types in the Vryn language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // === Literals ===
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),

    // === Identifier ===
    Identifier(String),

    // === Keywords ===
    Fn,
    Let,
    Mut,
    If,
    Else,
    Match,
    For,
    While,
    Loop,
    Break,
    Continue,
    Return,
    Struct,
    Enum,
    Trait,
    Impl,
    Pub,
    Use,
    Spawn,
    Async,
    Await,
    True,
    False,
    SelfKw,     // self
    In,
    As,
    Type,
    Const,
    Static,
    Mod,
    Where,
    Try,
    Catch,
    // Some, None, Ok, Err are now standard Identifiers

    // === Operators ===
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %
    Equal,          // =
    EqualEqual,     // ==
    NotEqual,       // !=
    Less,           // <
    Greater,        // >
    LessEqual,      // <=
    GreaterEqual,   // >=
    And,            // &&
    Or,             // ||
    Not,            // !
    Ampersand,      // &
    Pipe,           // |
    Caret,          // ^
    Tilde,          // ~
    ShiftLeft,      // <<
    ShiftRight,     // >>
    PlusEqual,      // +=
    MinusEqual,     // -=
    StarEqual,      // *=
    SlashEqual,     // /=
    PipeArrow,      // |>  (pipe operator!)
    QuestionDot,    // ?.  (safe call)
    Question,       // ?   (error propagation)
    DotDot,         // ..  (range)
    DotDotEqual,    // ..= (inclusive range)
    FatArrow,       // =>  (match arm)
    ThinArrow,      // ->  (return type)
    ColonColon,     // ::  (path separator)

    // === Delimiters ===
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    Comma,          // ,
    Semicolon,      // ;
    Colon,          // :
    Dot,            // .

    // === Special ===
    Newline,
    Eof,
}

/// A single token with its location in source code
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: String, line: usize, column: usize) -> Self {
        Token { kind, lexeme, line, column }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} '{}' at {}:{}", self.kind, self.lexeme, self.line, self.column)
    }
}

/// Look up if an identifier is actually a keyword
pub fn lookup_keyword(word: &str) -> Option<TokenKind> {
    match word {
        "fn"       => Some(TokenKind::Fn),
        "let"      => Some(TokenKind::Let),
        "mut"      => Some(TokenKind::Mut),
        "if"       => Some(TokenKind::If),
        "else"     => Some(TokenKind::Else),
        "match"    => Some(TokenKind::Match),
        "for"      => Some(TokenKind::For),
        "while"    => Some(TokenKind::While),
        "loop"     => Some(TokenKind::Loop),
        "break"    => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "return"   => Some(TokenKind::Return),
        "struct"   => Some(TokenKind::Struct),
        "enum"     => Some(TokenKind::Enum),
        "trait"    => Some(TokenKind::Trait),
        "impl"     => Some(TokenKind::Impl),
        "pub"      => Some(TokenKind::Pub),
        "use"      => Some(TokenKind::Use),
        "spawn"    => Some(TokenKind::Spawn),
        "async"    => Some(TokenKind::Async),
        "await"    => Some(TokenKind::Await),
        "true"     => Some(TokenKind::True),
        "false"    => Some(TokenKind::False),
        "self"     => Some(TokenKind::SelfKw),
        "in"       => Some(TokenKind::In),
        "as"       => Some(TokenKind::As),
        "type"     => Some(TokenKind::Type),
        "const"    => Some(TokenKind::Const),
        "static"   => Some(TokenKind::Static),
        "mod"      => Some(TokenKind::Mod),
        "where"    => Some(TokenKind::Where),
        "try"      => Some(TokenKind::Try),
        "catch"    => Some(TokenKind::Catch),
        _ => Option::None,
    }
}
