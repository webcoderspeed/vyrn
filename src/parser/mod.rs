pub mod ast;

use crate::lexer::token::{Token, TokenKind};
use ast::*;

/// The Vryn Parser — converts tokens into an AST
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    /// Parse the entire token stream into a Program
    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() { break; }
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    // ==========================================
    //            STATEMENT PARSING
    // ==========================================

    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.skip_newlines();

        match self.current_kind() {
            TokenKind::Fn => self.parse_function(),
            TokenKind::Let => self.parse_let(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Struct => self.parse_struct(),
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Break => { self.advance(); Ok(Statement::Break) }
            TokenKind::Continue => { self.advance(); Ok(Statement::Continue) }
            _ => {
                let expr = self.parse_expression()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    /// Parse: fn name(params) -> ReturnType { body }
    fn parse_function(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Fn)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftParen)?;

        let params = self.parse_params()?;
        self.expect(TokenKind::RightParen)?;

        let return_type = if self.check(&TokenKind::ThinArrow) {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Statement::Function { name, params, return_type, body })
    }

    /// Parse function parameters: (a: i32, b: str)
    fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RightParen) {
            return Ok(params);
        }

        loop {
            let name = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            let type_name = self.expect_identifier()?;
            params.push(Param { name, type_name });

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance(); // skip comma
        }

        Ok(params)
    }

    /// Parse: let [mut] name [: type] = value
    fn parse_let(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Let)?;

        let mutable = if self.check(&TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;

        let type_ann = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };

        self.expect(TokenKind::Equal)?;
        let value = self.parse_expression()?;

        Ok(Statement::Let { name, mutable, type_ann, value })
    }

    /// Parse: if condition { body } [else { body }]
    fn parse_if(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::If)?;
        let condition = Box::new(self.parse_expression()?);
        let then_body = self.parse_block()?;

        let else_body = if self.check(&TokenKind::Else) {
            self.advance();
            if self.check(&TokenKind::If) {
                // else if chain
                Some(vec![self.parse_if()?])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        Ok(Statement::If { condition, then_body, else_body })
    }

    /// Parse: while condition { body }
    fn parse_while(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::While)?;
        let condition = Box::new(self.parse_expression()?);
        let body = self.parse_block()?;
        Ok(Statement::While { condition, body })
    }

    /// Parse: for var in iterable { body }
    fn parse_for(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::For)?;
        let var = self.expect_identifier()?;
        self.expect(TokenKind::In)?;
        let iterable = Box::new(self.parse_expression()?);
        let body = self.parse_block()?;
        Ok(Statement::For { var, iterable, body })
    }

    /// Parse: return [expression]
    fn parse_return(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Return)?;
        self.skip_newlines();

        if self.check(&TokenKind::RightBrace) || self.is_at_end() {
            Ok(Statement::Return(None))
        } else {
            let expr = self.parse_expression()?;
            Ok(Statement::Return(Some(expr)))
        }
    }

    /// Parse: struct Name { field: Type, ... }
    fn parse_struct(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Struct)?;
        let name = self.expect_identifier()?;
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace)?;

        let mut fields = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenKind::RightBrace) {
            let field_name = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            let type_name = self.expect_identifier()?;
            fields.push(Field { name: field_name, type_name });

            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(Statement::Struct { name, fields })
    }

    /// Parse: enum Name { Variant1, Variant2(Type), ... }
    fn parse_enum(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Enum)?;
        let name = self.expect_identifier()?;
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace)?;

        let mut variants = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenKind::RightBrace) {
            let variant_name = self.expect_identifier()?;
            let mut variant_fields = Vec::new();

            if self.check(&TokenKind::LeftParen) {
                self.advance();
                while !self.check(&TokenKind::RightParen) {
                    variant_fields.push(self.expect_identifier()?);
                    if self.check(&TokenKind::Comma) { self.advance(); }
                }
                self.expect(TokenKind::RightParen)?;
            }

            variants.push(EnumVariant { name: variant_name, fields: variant_fields });

            self.skip_newlines();
            if self.check(&TokenKind::Comma) { self.advance(); }
            self.skip_newlines();
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(Statement::Enum { name, variants })
    }

    /// Parse a block: { statement* }
    fn parse_block(&mut self) -> Result<Vec<Statement>, String> {
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace)?;

        let mut stmts = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(stmts)
    }

    // ==========================================
    //           EXPRESSION PARSING
    //   (Pratt parser / precedence climbing)
    // ==========================================

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_pipe()
    }

    /// Pipe: expr |> expr
    fn parse_pipe(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_assignment()?;

        while self.check(&TokenKind::PipeArrow) {
            self.advance();
            let right = self.parse_assignment()?;
            left = Expression::Pipe {
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Assignment: target = value
    fn parse_assignment(&mut self) -> Result<Expression, String> {
        let expr = self.parse_or()?;

        if self.check(&TokenKind::Equal) {
            self.advance();
            let value = self.parse_expression()?;
            return Ok(Expression::Assign {
                target: Box::new(expr),
                value: Box::new(value),
            });
        }

        Ok(expr)
    }

    /// Logical OR: a || b
    fn parse_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and()?;

        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Logical AND: a && b
    fn parse_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_equality()?;

        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Equality: a == b, a != b
    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::EqualEqual => BinaryOperator::Eq,
                TokenKind::NotEqual => BinaryOperator::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Comparison: a < b, a >= b, etc.
    fn parse_comparison(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_range()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::Less => BinaryOperator::Less,
                TokenKind::Greater => BinaryOperator::Greater,
                TokenKind::LessEqual => BinaryOperator::LessEq,
                TokenKind::GreaterEqual => BinaryOperator::GreaterEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_range()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Range: a..b or a..=b
    fn parse_range(&mut self) -> Result<Expression, String> {
        let left = self.parse_addition()?;

        if self.check(&TokenKind::DotDot) {
            self.advance();
            let right = self.parse_addition()?;
            return Ok(Expression::Range {
                start: Box::new(left),
                end: Box::new(right),
                inclusive: false,
            });
        }

        if self.check(&TokenKind::DotDotEqual) {
            self.advance();
            let right = self.parse_addition()?;
            return Ok(Expression::Range {
                start: Box::new(left),
                end: Box::new(right),
                inclusive: true,
            });
        }

        Ok(left)
    }

    /// Addition: a + b, a - b
    fn parse_addition(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplication()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Multiplication: a * b, a / b, a % b
    fn parse_multiplication(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::Star => BinaryOperator::Mul,
                TokenKind::Slash => BinaryOperator::Div,
                TokenKind::Percent => BinaryOperator::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Unary: -x, !x
    fn parse_unary(&mut self) -> Result<Expression, String> {
        match self.current_kind() {
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOperator::Neg,
                    operand: Box::new(operand),
                })
            }
            TokenKind::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOperator::Not,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_call(),
        }
    }

    /// Function call / member access: foo(args), obj.field, arr[i]
    fn parse_call(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::LeftParen) {
                self.advance();
                let args = self.parse_args()?;
                self.expect(TokenKind::RightParen)?;
                expr = Expression::Call {
                    function: Box::new(expr),
                    args,
                };
            } else if self.check(&TokenKind::Dot) || self.check(&TokenKind::ColonColon) {
                self.advance();
                let member = self.expect_identifier()?;
                expr = Expression::MemberAccess {
                    object: Box::new(expr),
                    member,
                };
            } else if self.check(&TokenKind::LeftBracket) {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(TokenKind::RightBracket)?;
                expr = Expression::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.check(&TokenKind::LeftBrace) {
                // Check for Struct Init: Identifier { field: value }
                // We look ahead: { ident :
                let is_struct_init = if let Expression::Identifier(_) = &expr {
                    // Look ahead for Identifier then Colon, skipping Newlines
                    let mut k = self.pos + 1;
                    while k < self.tokens.len() && self.tokens[k].kind == TokenKind::Newline {
                        k += 1;
                    }
                    
                    if k < self.tokens.len() {
                         if let TokenKind::Identifier(_) = &self.tokens[k].kind {
                             // Found identifier, now look for colon
                             k += 1;
                             while k < self.tokens.len() && self.tokens[k].kind == TokenKind::Newline {
                                 k += 1;
                             }
                             if k < self.tokens.len() {
                                 std::mem::discriminant(&self.tokens[k].kind) == std::mem::discriminant(&TokenKind::Colon)
                             } else { false }
                         } else { false }
                    } else { false }
                } else {
                    false
                };

                if is_struct_init {
                    let name = if let Expression::Identifier(n) = expr { n } else { unreachable!() };
                    self.advance(); // consume '{'

                    let mut fields = Vec::new();
                    self.skip_newlines();

                    while !self.check(&TokenKind::RightBrace) {
                        let field_name = self.expect_identifier()?;
                        self.expect(TokenKind::Colon)?;
                        let val = self.parse_expression()?;
                        fields.push((field_name, val));

                        self.skip_newlines();
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        }
                        self.skip_newlines();
                    }

                    self.expect(TokenKind::RightBrace)?;
                    expr = Expression::StructInit {
                        name,
                        fields,
                    };
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse call arguments
    fn parse_args(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = Vec::new();

        if self.check(&TokenKind::RightParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);
            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        Ok(args)
    }

    /// Primary expressions: literals, identifiers, grouped expressions
    fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.current_kind() {
            TokenKind::IntLiteral(n) => {
                let val = n;
                self.advance();
                Ok(Expression::IntLiteral(val))
            }
            TokenKind::FloatLiteral(f) => {
                let val = f;
                self.advance();
                Ok(Expression::FloatLiteral(val))
            }
            TokenKind::StringLiteral(ref s) => {
                let val = s.clone();
                self.advance();
                Ok(Expression::StringLiteral(val))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::BoolLiteral(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::BoolLiteral(false))
            }
            TokenKind::BoolLiteral(b) => {
                let val = b;
                self.advance();
                Ok(Expression::BoolLiteral(val))
            }
            TokenKind::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(Expression::Identifier(name))
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RightParen)?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                while !self.check(&TokenKind::RightBracket) {
                    elements.push(self.parse_expression()?);
                    if self.check(&TokenKind::Comma) { self.advance(); }
                }
                self.expect(TokenKind::RightBracket)?;
                Ok(Expression::Array(elements))
            }
            TokenKind::Match => {
                self.parse_match_expression()
            }
            TokenKind::Pipe => {
                self.parse_lambda()
            }
            _ => {
                let tok = &self.tokens[self.pos];
                Err(format!(
                    "Unexpected token {:?} '{}' at line {}, column {}",
                    tok.kind, tok.lexeme, tok.line, tok.column
                ))
            }
        }
    }

    /// Parse match expression
    fn parse_match_expression(&mut self) -> Result<Expression, String> {
        self.expect(TokenKind::Match)?;
        let value = Box::new(self.parse_expression()?);
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace)?;

        let mut arms = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenKind::RightBrace) {
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expression()?;
            arms.push(MatchArm { pattern, body });

            self.skip_newlines();
            if self.check(&TokenKind::Comma) { self.advance(); }
            self.skip_newlines();
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(Expression::Match { value, arms })
    }

    /// Parse a pattern (for match arms)
    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        self.skip_newlines();
        match self.current_kind() {
            TokenKind::IntLiteral(n) => {
                let val = n;
                self.advance();
                Ok(Pattern::Literal(Expression::IntLiteral(val)))
            }
            TokenKind::StringLiteral(ref s) => {
                let val = s.clone();
                self.advance();
                Ok(Pattern::Literal(Expression::StringLiteral(val)))
            }
            TokenKind::True => { self.advance(); Ok(Pattern::Literal(Expression::BoolLiteral(true))) }
            TokenKind::False => { self.advance(); Ok(Pattern::Literal(Expression::BoolLiteral(false))) }
            TokenKind::Identifier(ref name) if name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            TokenKind::Identifier(ref name) => {
                let mut name = name.clone();
                self.advance();

                // Check for Enum::Variant syntax
                if self.check(&TokenKind::ColonColon) {
                    self.advance();
                    let variant = self.expect_identifier()?;
                    // Store as "Enum::Variant"
                    name = format!("{}::{}", name, variant);
                }

                // Check for enum variant: Name(fields)
                if self.check(&TokenKind::LeftParen) {
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RightParen) {
                        fields.push(self.parse_pattern()?);
                        if self.check(&TokenKind::Comma) { self.advance(); }
                    }
                    self.expect(TokenKind::RightParen)?;
                    Ok(Pattern::EnumVariant { name, fields })
                } else {
                    Ok(Pattern::Identifier(name))
                }
            }
            _ => {
                let tok = &self.tokens[self.pos];
                Err(format!("Expected pattern, found {:?} at line {}", tok.kind, tok.line))
            }
        }
    }

    /// Parse lambda: |x, y| expr
    fn parse_lambda(&mut self) -> Result<Expression, String> {
        self.expect(TokenKind::Pipe)?;
        let mut params = Vec::new();

        if !self.check(&TokenKind::Pipe) {
            loop {
                params.push(self.expect_identifier()?);
                if !self.check(&TokenKind::Comma) { break; }
                self.advance();
            }
        }

        self.expect(TokenKind::Pipe)?;
        let body = Box::new(self.parse_expression()?);

        Ok(Expression::Lambda { params, body })
    }

    // ==========================================
    //             HELPER METHODS
    // ==========================================

    fn current_kind(&self) -> TokenKind {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].kind.clone()
        } else {
            TokenKind::Eof
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.pos < self.tokens.len() {
            std::mem::discriminant(&self.tokens[self.pos].kind) == std::mem::discriminant(kind)
        } else {
            false
        }
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: TokenKind) -> Result<(), String> {
        self.skip_newlines();
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            let tok = if self.pos < self.tokens.len() {
                &self.tokens[self.pos]
            } else {
                self.tokens.last().unwrap()
            };
            Err(format!(
                "Expected {:?}, found {:?} '{}' at line {}, column {}",
                expected, tok.kind, tok.lexeme, tok.line, tok.column
            ))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        self.skip_newlines();
        if let TokenKind::Identifier(ref name) = self.current_kind() {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            let tok = &self.tokens[self.pos];
            Err(format!(
                "Expected identifier, found {:?} '{}' at line {}, column {}",
                tok.kind, tok.lexeme, tok.line, tok.column
            ))
        }
    }

    fn skip_newlines(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::Newline {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == TokenKind::Eof
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_source(source: &str) -> Result<Program, String> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_let_statement() {
        let program = parse_source("let x = 42").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Let { name, mutable, value, .. } => {
                assert_eq!(name, "x");
                assert!(!mutable);
                match value {
                    Expression::IntLiteral(42) => {}
                    _ => panic!("Expected IntLiteral(42)"),
                }
            }
            _ => panic!("Expected Let statement"),
        }
    }

    #[test]
    fn test_function() {
        let program = parse_source("fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();
        match &program.statements[0] {
            Statement::Function { name, params, return_type, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(return_type.as_deref(), Some("i32"));
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_if_else() {
        let program = parse_source("if x > 0 { println(x) } else { println(y) }").unwrap();
        match &program.statements[0] {
            Statement::If { else_body, .. } => {
                assert!(else_body.is_some());
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_pipe_operator() {
        let program = parse_source("let r = data |> transform |> filter").unwrap();
        match &program.statements[0] {
            Statement::Let { value, .. } => {
                match value {
                    Expression::Pipe { .. } => {}
                    _ => panic!("Expected Pipe expression"),
                }
            }
            _ => panic!("Expected Let"),
        }
    }

    #[test]
    fn test_struct() {
        let src = "struct Point { x: f64, y: f64 }";
        let program = parse_source(src).unwrap();
        match &program.statements[0] {
            Statement::Struct { name, fields } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
            }
            _ => panic!("Expected Struct"),
        }
    }

    #[test]
    fn test_hello_world() {
        let src = r#"fn main() { println("Hello, World!") }"#;
        let program = parse_source(src).unwrap();
        assert_eq!(program.statements.len(), 1);
    }
}
