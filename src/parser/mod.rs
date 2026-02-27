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
            TokenKind::Fun | TokenKind::Fn => self.parse_function(),
            TokenKind::Var => self.parse_var(),
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

    /// Parse: fun name(params) -> ReturnType { body }
    /// Also accepts: fn (backward compat alias)
    /// Type annotations are OPTIONAL: fun add(a, b) { ... } or fun add(a: int, b: int) -> int { ... }
    fn parse_function(&mut self) -> Result<Statement, String> {
        // Accept both `fun` and `fn`
        if self.check(&TokenKind::Fun) || self.check(&TokenKind::Fn) {
            self.advance();
        } else {
            return Err("Expected 'fun' or 'fn'".to_string());
        }
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

    /// Parse function parameters — type annotations are OPTIONAL!
    /// fun add(a, b) { ... }            — no types (inferred as "any")
    /// fun add(a: int, b: int) { ... }  — with types
    /// fun add(a, b: int) { ... }       — mixed
    fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RightParen) {
            return Ok(params);
        }

        loop {
            let name = self.expect_identifier()?;
            let type_name = if self.check(&TokenKind::Colon) {
                self.advance();
                self.expect_identifier()?
            } else {
                "any".to_string()  // type inferred at runtime
            };
            params.push(Param { name, type_name });

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance(); // skip comma
        }

        Ok(params)
    }

    /// Parse: var name [: type] = value  (MUTABLE variable — the simple way!)
    fn parse_var(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Var)?;
        let name = self.expect_identifier()?;

        let type_ann = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };

        self.expect(TokenKind::Equal)?;
        let value = self.parse_expression()?;

        Ok(Statement::Let { name, mutable: true, type_ann, value })
    }

    /// Parse: let [mut] name [: type] = value  (immutable by default, mut for backward compat)
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

    /// Parse: if [let pattern = expr] condition { body } [else { body }]
    fn parse_if(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::If)?;

        // Check for "let" keyword to distinguish if-let from regular if
        if self.check(&TokenKind::Let) {
            self.advance();
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::Equal)?;
            let expr = Box::new(self.parse_expression()?);
            let then_body = self.parse_block()?;

            let else_body = if self.check(&TokenKind::Else) {
                self.advance();
                if self.check(&TokenKind::If) {
                    Some(vec![self.parse_if()?])
                } else {
                    Some(self.parse_block()?)
                }
            } else {
                None
            };

            Ok(Statement::IfLet { pattern, expr, then_body, else_body })
        } else {
            // Regular if statement
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
    }

    /// Parse: while condition { body }
    fn parse_while(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::While)?;

        // Check for "let" keyword to distinguish while-let from regular while
        if self.check(&TokenKind::Let) {
            self.advance();
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::Equal)?;
            let expr = Box::new(self.parse_expression()?);
            let body = self.parse_block()?;
            Ok(Statement::WhileLet { pattern, expr, body })
        } else {
            // Regular while statement
            let condition = Box::new(self.parse_expression()?);
            let body = self.parse_block()?;
            Ok(Statement::While { condition, body })
        }
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
            } else if self.check(&TokenKind::Question) {
                self.advance();
                expr = Expression::QuestionMark {
                    expr: Box::new(expr),
                };
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
            TokenKind::Try => {
                self.parse_try_catch()
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

    /// Parse match expression (now handles guards in patterns)
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

    /// Parse a pattern (for match arms and if-let)
    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        self.parse_or_pattern()
    }

    /// Parse or-pattern: pattern | pattern | pattern
    fn parse_or_pattern(&mut self) -> Result<Pattern, String> {
        let mut patterns = vec![self.parse_pattern_base()?];

        while self.check(&TokenKind::Or) {
            self.advance();
            patterns.push(self.parse_pattern_base()?);
        }

        if patterns.len() == 1 {
            Ok(patterns.into_iter().next().unwrap())
        } else {
            Ok(Pattern::Or(patterns))
        }
    }

    /// Parse a single pattern (not or-patterns): literals, identifiers, tuples, guards
    fn parse_pattern_base(&mut self) -> Result<Pattern, String> {
        self.skip_newlines();
        let pattern = match self.current_kind() {
            TokenKind::IntLiteral(n) => {
                let val = n;
                self.advance();
                Pattern::Literal(Expression::IntLiteral(val))
            }
            TokenKind::StringLiteral(ref s) => {
                let val = s.clone();
                self.advance();
                Pattern::Literal(Expression::StringLiteral(val))
            }
            TokenKind::True => { self.advance(); Pattern::Literal(Expression::BoolLiteral(true)) }
            TokenKind::False => { self.advance(); Pattern::Literal(Expression::BoolLiteral(false)) }
            TokenKind::Identifier(ref name) if name == "_" => {
                self.advance();
                Pattern::Wildcard
            }
            TokenKind::LeftParen => {
                // Tuple pattern
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        elements.push(self.parse_pattern()?);
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.expect(TokenKind::RightParen)?;
                Pattern::Tuple(elements)
            }
            TokenKind::Identifier(ref name) => {
                let mut name = name.clone();
                self.advance();

                // Check for Enum::Variant or Struct pattern
                if self.check(&TokenKind::ColonColon) {
                    self.advance();
                    let variant = self.expect_identifier()?;
                    name = format!("{}::{}", name, variant);
                }

                // Check for struct pattern: Name { x, y } or Name { x: pat, y: pat }
                if self.check(&TokenKind::LeftBrace) {
                    // Look ahead to distinguish struct pattern from regular brace
                    let is_struct_pattern = {
                        let mut k = self.pos + 1;
                        while k < self.tokens.len() && self.tokens[k].kind == TokenKind::Newline {
                            k += 1;
                        }

                        if k < self.tokens.len() {
                            match &self.tokens[k].kind {
                                TokenKind::Identifier(_) => true,
                                TokenKind::RightBrace => true,
                                _ => false,
                            }
                        } else {
                            false
                        }
                    };

                    if is_struct_pattern {
                        self.advance(); // consume '{'
                        let mut fields = Vec::new();
                        self.skip_newlines();

                        while !self.check(&TokenKind::RightBrace) {
                            let field_name = self.expect_identifier()?;
                            let field_pattern = if self.check(&TokenKind::Colon) {
                                self.advance();
                                self.parse_pattern()?
                            } else {
                                // Shorthand: x means x: x
                                Pattern::Identifier(field_name.clone())
                            };
                            fields.push((field_name, field_pattern));

                            self.skip_newlines();
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                            }
                            self.skip_newlines();
                        }

                        self.expect(TokenKind::RightBrace)?;
                        Pattern::Struct { name, fields }
                    } else {
                        // Regular identifier
                        Pattern::Identifier(name)
                    }
                } else if self.check(&TokenKind::LeftParen) {
                    // Enum variant with data
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RightParen) {
                        fields.push(self.parse_pattern()?);
                        if self.check(&TokenKind::Comma) { self.advance(); }
                    }
                    self.expect(TokenKind::RightParen)?;
                    Pattern::EnumVariant { name, fields }
                } else {
                    Pattern::Identifier(name)
                }
            }
            _ => {
                let tok = &self.tokens[self.pos];
                return Err(format!("Expected pattern, found {:?} at line {}", tok.kind, tok.line));
            }
        };

        // Check for guard clause: pattern if condition
        if self.check(&TokenKind::If) {
            self.advance();
            let condition = Box::new(self.parse_expression()?);
            Ok(Pattern::Guard {
                pattern: Box::new(pattern),
                condition,
            })
        } else {
            Ok(pattern)
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

    /// Parse try-catch: try { ... } catch var { ... }
    fn parse_try_catch(&mut self) -> Result<Expression, String> {
        self.expect(TokenKind::Try)?;
        let try_body = self.parse_block()?;

        self.expect(TokenKind::Catch)?;
        let catch_var = self.expect_identifier()?;
        let catch_body = self.parse_block()?;

        Ok(Expression::TryCatch {
            try_body,
            catch_var,
            catch_body,
        })
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
        // New simple syntax: fun with optional types
        let program = parse_source("fun add(a, b) { a + b }").unwrap();
        match &program.statements[0] {
            Statement::Function { name, params, return_type, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].type_name, "any"); // optional = inferred
                assert_eq!(return_type, &None);
            }
            _ => panic!("Expected Function"),
        }

        // Old syntax still works (backward compat)
        let program2 = parse_source("fn add(a: int, b: int) -> int { a + b }").unwrap();
        match &program2.statements[0] {
            Statement::Function { name, params, return_type, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].type_name, "int");
                assert_eq!(return_type.as_deref(), Some("int"));
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
        // fun works
        let src = r#"fun main() { println("Hello, World!") }"#;
        let program = parse_source(src).unwrap();
        assert_eq!(program.statements.len(), 1);

        // fn also works (backward compat)
        let src2 = r#"fn main() { println("Hello, World!") }"#;
        let program2 = parse_source(src2).unwrap();
        assert_eq!(program2.statements.len(), 1);
    }

    #[test]
    fn test_var_syntax() {
        // var = mutable variable (replaces let mut)
        let program = parse_source("var count = 0").unwrap();
        match &program.statements[0] {
            Statement::Let { name, mutable, .. } => {
                assert_eq!(name, "count");
                assert!(*mutable);
            }
            _ => panic!("Expected Let (from var)"),
        }
    }
}
