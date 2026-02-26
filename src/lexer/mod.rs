pub mod token;

use token::{Token, TokenKind, lookup_keyword};

/// The Vryn Lexer — converts source code into tokens
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
        }
    }

    /// Tokenize the entire source code
    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }
            self.scan_token()?;
        }

        self.tokens.push(Token::new(
            TokenKind::Eof,
            String::new(),
            self.line,
            self.column,
        ));

        Ok(self.tokens.clone())
    }

    fn scan_token(&mut self) -> Result<(), String> {
        let ch = self.current();
        let line = self.line;
        let col = self.column;

        match ch {
            // === Single character tokens ===
            '(' => { self.add_token(TokenKind::LeftParen, "(", line, col); self.advance(); }
            ')' => { self.add_token(TokenKind::RightParen, ")", line, col); self.advance(); }
            '{' => { self.add_token(TokenKind::LeftBrace, "{", line, col); self.advance(); }
            '}' => { self.add_token(TokenKind::RightBrace, "}", line, col); self.advance(); }
            '[' => { self.add_token(TokenKind::LeftBracket, "[", line, col); self.advance(); }
            ']' => { self.add_token(TokenKind::RightBracket, "]", line, col); self.advance(); }
            ',' => { self.add_token(TokenKind::Comma, ",", line, col); self.advance(); }
            ';' => { self.add_token(TokenKind::Semicolon, ";", line, col); self.advance(); }
            '~' => { self.add_token(TokenKind::Tilde, "~", line, col); self.advance(); }
            '^' => { self.add_token(TokenKind::Caret, "^", line, col); self.advance(); }
            '%' => { self.add_token(TokenKind::Percent, "%", line, col); self.advance(); }

            // === Newlines ===
            '\n' => {
                self.add_token(TokenKind::Newline, "\\n", line, col);
                self.advance();
                self.line += 1;
                self.column = 1;
                return Ok(());
            }

            // === Two-char tokens ===
            '+' => {
                self.advance();
                if self.match_char('=') {
                    self.add_token(TokenKind::PlusEqual, "+=", line, col);
                } else {
                    self.add_token(TokenKind::Plus, "+", line, col);
                }
            }
            '-' => {
                self.advance();
                if self.match_char('>') {
                    self.add_token(TokenKind::ThinArrow, "->", line, col);
                } else if self.match_char('=') {
                    self.add_token(TokenKind::MinusEqual, "-=", line, col);
                } else {
                    self.add_token(TokenKind::Minus, "-", line, col);
                }
            }
            '*' => {
                self.advance();
                if self.match_char('=') {
                    self.add_token(TokenKind::StarEqual, "*=", line, col);
                } else {
                    self.add_token(TokenKind::Star, "*", line, col);
                }
            }
            '/' => {
                self.advance();
                if self.match_char('/') {
                    // Line comment — skip until end of line
                    while !self.is_at_end() && self.current() != '\n' {
                        self.advance();
                    }
                } else if self.match_char('*') {
                    // Block comment — skip until */
                    self.skip_block_comment()?;
                } else if self.match_char('=') {
                    self.add_token(TokenKind::SlashEqual, "/=", line, col);
                } else {
                    self.add_token(TokenKind::Slash, "/", line, col);
                }
            }
            '=' => {
                self.advance();
                if self.match_char('=') {
                    self.add_token(TokenKind::EqualEqual, "==", line, col);
                } else if self.match_char('>') {
                    self.add_token(TokenKind::FatArrow, "=>", line, col);
                } else {
                    self.add_token(TokenKind::Equal, "=", line, col);
                }
            }
            '!' => {
                self.advance();
                if self.match_char('=') {
                    self.add_token(TokenKind::NotEqual, "!=", line, col);
                } else {
                    self.add_token(TokenKind::Not, "!", line, col);
                }
            }
            '<' => {
                self.advance();
                if self.match_char('=') {
                    self.add_token(TokenKind::LessEqual, "<=", line, col);
                } else if self.match_char('<') {
                    self.add_token(TokenKind::ShiftLeft, "<<", line, col);
                } else {
                    self.add_token(TokenKind::Less, "<", line, col);
                }
            }
            '>' => {
                self.advance();
                if self.match_char('=') {
                    self.add_token(TokenKind::GreaterEqual, ">=", line, col);
                } else if self.match_char('>') {
                    self.add_token(TokenKind::ShiftRight, ">>", line, col);
                } else {
                    self.add_token(TokenKind::Greater, ">", line, col);
                }
            }
            '&' => {
                self.advance();
                if self.match_char('&') {
                    self.add_token(TokenKind::And, "&&", line, col);
                } else {
                    self.add_token(TokenKind::Ampersand, "&", line, col);
                }
            }
            '|' => {
                self.advance();
                if self.match_char('|') {
                    self.add_token(TokenKind::Or, "||", line, col);
                } else if self.match_char('>') {
                    self.add_token(TokenKind::PipeArrow, "|>", line, col);
                } else {
                    self.add_token(TokenKind::Pipe, "|", line, col);
                }
            }
            ':' => {
                self.advance();
                if self.match_char(':') {
                    self.add_token(TokenKind::ColonColon, "::", line, col);
                } else {
                    self.add_token(TokenKind::Colon, ":", line, col);
                }
            }
            '.' => {
                self.advance();
                if self.match_char('.') {
                    if self.match_char('=') {
                        self.add_token(TokenKind::DotDotEqual, "..=", line, col);
                    } else {
                        self.add_token(TokenKind::DotDot, "..", line, col);
                    }
                } else {
                    self.add_token(TokenKind::Dot, ".", line, col);
                }
            }
            '?' => {
                self.advance();
                if self.match_char('.') {
                    self.add_token(TokenKind::QuestionDot, "?.", line, col);
                } else {
                    self.add_token(TokenKind::Question, "?", line, col);
                }
            }

            // === String Literals ===
            '"' => self.scan_string()?,

            // === Number Literals ===
            c if c.is_ascii_digit() => self.scan_number()?,

            // === Identifiers & Keywords ===
            c if c.is_alphabetic() || c == '_' => self.scan_identifier(),

            _ => {
                return Err(format!(
                    "Unexpected character '{}' at line {}, column {}",
                    ch, self.line, self.column
                ));
            }
        }

        Ok(())
    }

    /// Scan a string literal with interpolation support
    fn scan_string(&mut self) -> Result<(), String> {
        let line = self.line;
        let col = self.column;
        self.advance(); // skip opening "

        let mut value = String::new();

        while !self.is_at_end() && self.current() != '"' {
            if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(format!("Unterminated string at line {}, column {}", line, col));
                }
                match self.current() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '{' => value.push('{'),
                    '0' => value.push('\0'),
                    _ => {
                        return Err(format!(
                            "Unknown escape sequence '\\{}' at line {}, column {}",
                            self.current(), self.line, self.column
                        ));
                    }
                }
                self.advance();
            } else if self.current() == '\n' {
                value.push('\n');
                self.advance();
                self.line += 1;
                self.column = 1;
                continue;
            } else {
                value.push(self.current());
                self.advance();
            }
        }

        if self.is_at_end() {
            return Err(format!("Unterminated string at line {}, column {}", line, col));
        }

        self.advance(); // skip closing "

        self.add_token(
            TokenKind::StringLiteral(value.clone()),
            &format!("\"{}\"", value),
            line,
            col,
        );

        Ok(())
    }

    /// Scan a number literal (integer or float)
    fn scan_number(&mut self) -> Result<(), String> {
        let line = self.line;
        let col = self.column;
        let mut num_str = String::new();
        let mut is_float = false;

        // Check for hex (0x), binary (0b), octal (0o)
        if self.current() == '0' && !self.is_at_end() {
            num_str.push(self.current());
            self.advance();

            if !self.is_at_end() {
                match self.current() {
                    'x' | 'X' => {
                        num_str.push(self.current());
                        self.advance();
                        while !self.is_at_end() && (self.current().is_ascii_hexdigit() || self.current() == '_') {
                            if self.current() != '_' { num_str.push(self.current()); }
                            self.advance();
                        }
                        let clean: String = num_str[2..].to_string();
                        let val = i64::from_str_radix(&clean, 16)
                            .map_err(|e| format!("Invalid hex number at line {}: {}", line, e))?;
                        self.add_token(TokenKind::IntLiteral(val), &num_str, line, col);
                        return Ok(());
                    }
                    'b' | 'B' => {
                        num_str.push(self.current());
                        self.advance();
                        while !self.is_at_end() && (self.current() == '0' || self.current() == '1' || self.current() == '_') {
                            if self.current() != '_' { num_str.push(self.current()); }
                            self.advance();
                        }
                        let clean: String = num_str[2..].to_string();
                        let val = i64::from_str_radix(&clean, 2)
                            .map_err(|e| format!("Invalid binary number at line {}: {}", line, e))?;
                        self.add_token(TokenKind::IntLiteral(val), &num_str, line, col);
                        return Ok(());
                    }
                    'o' | 'O' => {
                        num_str.push(self.current());
                        self.advance();
                        while !self.is_at_end() && ((self.current() >= '0' && self.current() <= '7') || self.current() == '_') {
                            if self.current() != '_' { num_str.push(self.current()); }
                            self.advance();
                        }
                        let clean: String = num_str[2..].to_string();
                        let val = i64::from_str_radix(&clean, 8)
                            .map_err(|e| format!("Invalid octal number at line {}: {}", line, e))?;
                        self.add_token(TokenKind::IntLiteral(val), &num_str, line, col);
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        // Regular decimal number
        if num_str.is_empty() || num_str == "0" {
            if num_str.is_empty() {
                num_str.push(self.current());
                self.advance();
            }
            while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '_') {
                if self.current() != '_' { num_str.push(self.current()); }
                self.advance();
            }
        }

        // Check for float
        if !self.is_at_end() && self.current() == '.' && self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
            is_float = true;
            num_str.push('.');
            self.advance(); // skip .
            while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '_') {
                if self.current() != '_' { num_str.push(self.current()); }
                self.advance();
            }
        }

        if is_float {
            let val: f64 = num_str.parse()
                .map_err(|e| format!("Invalid float at line {}: {}", line, e))?;
            self.add_token(TokenKind::FloatLiteral(val), &num_str, line, col);
        } else {
            let val: i64 = num_str.parse()
                .map_err(|e| format!("Invalid integer at line {}: {}", line, e))?;
            self.add_token(TokenKind::IntLiteral(val), &num_str, line, col);
        }

        Ok(())
    }

    /// Scan an identifier or keyword
    fn scan_identifier(&mut self) {
        let line = self.line;
        let col = self.column;
        let mut ident = String::new();

        while !self.is_at_end() && (self.current().is_alphanumeric() || self.current() == '_') {
            ident.push(self.current());
            self.advance();
        }

        // Check if it's a keyword
        let kind = if let Some(kw) = lookup_keyword(&ident) {
            kw
        } else if ident == "true" {
            TokenKind::BoolLiteral(true)
        } else if ident == "false" {
            TokenKind::BoolLiteral(false)
        } else {
            TokenKind::Identifier(ident.clone())
        };

        self.add_token(kind, &ident, line, col);
    }

    // === Helper methods ===

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.current() {
                ' ' | '\t' | '\r' => { self.advance(); }
                _ => break,
            }
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), String> {
        let line = self.line;
        let mut depth = 1;

        while !self.is_at_end() && depth > 0 {
            if self.current() == '/' && self.peek_next() == Some('*') {
                depth += 1;
                self.advance();
                self.advance();
            } else if self.current() == '*' && self.peek_next() == Some('/') {
                depth -= 1;
                self.advance();
                self.advance();
            } else {
                if self.current() == '\n' {
                    self.line += 1;
                    self.column = 0;
                }
                self.advance();
            }
        }

        if depth > 0 {
            return Err(format!("Unterminated block comment starting at line {}", line));
        }

        Ok(())
    }

    fn current(&self) -> char {
        self.source[self.pos]
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.source.len() {
            Some(self.source[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
        self.column += 1;
    }

    fn match_char(&mut self, expected: char) -> bool {
        if !self.is_at_end() && self.current() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn add_token(&mut self, kind: TokenKind, lexeme: &str, line: usize, column: usize) {
        self.tokens.push(Token::new(kind, lexeme.to_string(), line, column));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let source = r#"fn main() { println("Hello, World!") }"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("main".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::LeftParen);
        assert_eq!(tokens[3].kind, TokenKind::RightParen);
        assert_eq!(tokens[4].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[5].kind, TokenKind::Identifier("println".to_string()));
    }

    #[test]
    fn test_variables() {
        let source = r#"let mut x = 42"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Mut);
        assert_eq!(tokens[2].kind, TokenKind::Identifier("x".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::Equal);
        assert_eq!(tokens[4].kind, TokenKind::IntLiteral(42));
    }

    #[test]
    fn test_operators() {
        let source = "|> -> => == != <= >= && || :: .. ..=";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::PipeArrow);
        assert_eq!(tokens[1].kind, TokenKind::ThinArrow);
        assert_eq!(tokens[2].kind, TokenKind::FatArrow);
        assert_eq!(tokens[3].kind, TokenKind::EqualEqual);
        assert_eq!(tokens[4].kind, TokenKind::NotEqual);
        assert_eq!(tokens[5].kind, TokenKind::LessEqual);
        assert_eq!(tokens[6].kind, TokenKind::GreaterEqual);
        assert_eq!(tokens[7].kind, TokenKind::And);
        assert_eq!(tokens[8].kind, TokenKind::Or);
        assert_eq!(tokens[9].kind, TokenKind::ColonColon);
        assert_eq!(tokens[10].kind, TokenKind::DotDot);
        assert_eq!(tokens[11].kind, TokenKind::DotDotEqual);
    }

    #[test]
    fn test_float() {
        let source = "3.14";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::FloatLiteral(3.14));
    }

    #[test]
    fn test_hex_binary_octal() {
        let mut lexer = Lexer::new("0xFF");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(255));

        let mut lexer = Lexer::new("0b1010");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(10));

        let mut lexer = Lexer::new("0o17");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(15));
    }

    #[test]
    fn test_string_escape() {
        let source = r#""hello\nworld""#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral("hello\nworld".to_string()));
    }

    #[test]
    fn test_comments_skipped() {
        let source = "let x = 5 // this is a comment\nlet y = 10";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        // Should have: let, x, =, 5, newline, let, y, =, 10, EOF
        let non_newline: Vec<_> = tokens.iter()
            .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
            .collect();
        assert_eq!(non_newline.len(), 8); // let x = 5 let y = 10
    }

    #[test]
    fn test_pipe_operator() {
        let source = "data |> filter |> map";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier("data".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::PipeArrow);
        assert_eq!(tokens[2].kind, TokenKind::Identifier("filter".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::PipeArrow);
        assert_eq!(tokens[4].kind, TokenKind::Identifier("map".to_string()));
    }

    #[test]
    fn test_full_function() {
        let source = r#"fn add(a: i32, b: i32) -> i32 {
    a + b
}"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("add".to_string()));
        // Should parse without errors
        assert!(tokens.last().unwrap().kind == TokenKind::Eof);
    }
}
