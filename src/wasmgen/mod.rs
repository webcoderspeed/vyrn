/// WebAssembly Text Format (WAT) code generator for Vryn
/// Generates WebAssembly code from Vryn AST

use crate::parser::ast::{Statement, Expression, BinaryOperator, Program};
use std::collections::HashMap;

/// WebAssembly Text Format (WAT) code generator
pub struct WasmCodeGen {
    output: String,
    locals: HashMap<String, usize>,
    local_count: usize,
    functions: Vec<String>,
    indent_level: usize,
}

impl WasmCodeGen {
    pub fn new() -> Self {
        WasmCodeGen {
            output: String::new(),
            locals: HashMap::new(),
            local_count: 0,
            functions: Vec::new(),
            indent_level: 0,
        }
    }

    fn indent(&self) -> String {
        "  ".repeat(self.indent_level)
    }

    pub fn generate(&mut self, program: &Program) -> String {
        self.output.push_str("(module\n");
        
        for stmt in &program.statements {
            self.gen_statement(stmt);
        }
        
        self.output.push_str(")\n");
        self.output.clone()
    }

    fn gen_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Function { name, params, body, .. } => {
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                self.gen_function(name, &param_names, body);
            }
            Statement::Expression(expr) => {
                self.gen_expr(expr);
            }
            Statement::If { condition: _, then_body, else_body } => {
                self.indent_level += 1;
                self.output.push_str(&self.indent());
                self.output.push_str("(if\n");
                self.indent_level += 1;
                self.output.push_str(&self.indent());
                self.output.push_str("(then\n");
                
                for stmt in then_body {
                    self.gen_statement(stmt);
                }
                
                self.output.push_str(&self.indent());
                self.output.push_str(")\n");
                
                if let Some(else_stmts) = else_body {
                    self.output.push_str(&self.indent());
                    self.output.push_str("(else\n");
                    self.indent_level += 1;
                    
                    for stmt in else_stmts {
                        self.gen_statement(stmt);
                    }
                    
                    self.indent_level -= 1;
                    self.output.push_str(&self.indent());
                    self.output.push_str(")\n");
                }
                
                self.indent_level -= 1;
                self.output.push_str(&self.indent());
                self.output.push_str(")\n");
                self.indent_level -= 1;
            }
            Statement::While { condition, body } => {
                self.indent_level += 1;
                self.output.push_str(&self.indent());
                self.output.push_str("(block $break\n");
                self.indent_level += 1;
                self.output.push_str(&self.indent());
                self.output.push_str("(loop $continue\n");
                self.indent_level += 1;
                
                // Generate condition
                self.gen_expr(condition);
                
                self.output.push_str(&self.indent());
                self.output.push_str("i32.eqz\n");
                self.indent_level += 1;
                self.output.push_str(&self.indent());
                self.output.push_str("(br_if $break)\n");
                self.indent_level -= 1;
                
                for stmt in body {
                    self.gen_statement(stmt);
                }
                
                self.output.push_str(&self.indent());
                self.output.push_str("(br $continue)\n");
                
                self.indent_level -= 1;
                self.output.push_str(&self.indent());
                self.output.push_str(")\n");
                self.indent_level -= 1;
                self.output.push_str(&self.indent());
                self.output.push_str(")\n");
                self.indent_level -= 1;
            }
            Statement::For { var, iterable: _, body } => {
                // Generate WAT for-loop structure using block and loop
                self.indent_level += 1;
                self.output.push_str(&self.indent());
                self.output.push_str(&format!("(block $break_{}\n", var));
                self.indent_level += 1;
                self.output.push_str(&self.indent());
                self.output.push_str(&format!("(loop $continue_{}\n", var));
                self.indent_level += 1;
                
                // Declare loop variable
                self.locals.insert(var.clone(), self.local_count);
                self.local_count += 1;
                
                for stmt in body {
                    self.gen_statement(stmt);
                }
                
                self.output.push_str(&self.indent());
                self.output.push_str(&format!("(br $continue_{})\n", var));
                
                self.indent_level -= 1;
                self.output.push_str(&self.indent());
                self.output.push_str(")\n");
                self.indent_level -= 1;
                self.output.push_str(&self.indent());
                self.output.push_str(")\n");
                self.indent_level -= 1;
            }
            _ => {}
        }
    }

    fn gen_function(&mut self, name: &str, params: &[String], body: &[Statement]) {
        let mut func_str = format!("  (func ${}", name);
        
        // Add parameters
        for param in params {
            func_str.push_str(&format!(" (param {} i32)", param));
        }
        
        // Add result type
        func_str.push_str(" (result i32)\n");
        
        // Reset locals
        self.locals.clear();
        self.local_count = params.len();
        
        // Add parameters to locals
        for (i, param) in params.iter().enumerate() {
            self.locals.insert(param.clone(), i);
        }
        
        // Generate body
        for stmt in body {
            match stmt {
                Statement::Expression(expr) => {
                    func_str.push_str(&self.gen_expr_str(expr));
                }
                _ => {}
            }
        }
        
        func_str.push_str("  )\n");
        self.functions.push(func_str);
    }

    fn gen_expr(&mut self, expr: &Expression) {
        let expr_str = self.gen_expr_str(expr);
        self.output.push_str(&expr_str);
    }

    fn gen_expr_str(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::IntLiteral(n) => {
                format!("    i32.const {}\n", n)
            }
            Expression::FloatLiteral(f) => {
                format!("    f32.const {}\n", f)
            }
            Expression::StringLiteral(s) => {
                // For now, represent strings as comments
                format!("    ;; string literal: \"{}\"\n", s)
            }
            Expression::BoolLiteral(b) => {
                let val = if *b { 1 } else { 0 };
                format!("    i32.const {}\n", val)
            }
            Expression::Identifier(name) => {
                if let Some(&idx) = self.locals.get(name) {
                    format!("    local.get {}\n", idx)
                } else {
                    format!("    ;; undefined variable: {}\n", name)
                }
            }
            Expression::BinaryOp { left, op, right } => {
                self.gen_binary_op(left, op, right)
            }
            Expression::UnaryOp { op, operand } => {
                let mut result = self.gen_expr_str(operand);
                let op_str = match op {
                    crate::parser::ast::UnaryOperator::Neg => "i32.const -1\ni32.mul",
                    crate::parser::ast::UnaryOperator::Not => "i32.eqz",
                };
                result.push_str(&format!("    {}\n", op_str));
                result
            }
            Expression::Call { function, args } => {
                let mut result = String::new();
                
                for arg in args {
                    result.push_str(&self.gen_expr_str(arg));
                }
                
                if let Expression::Identifier(func_name) = function.as_ref() {
                    result.push_str(&format!("    call ${}\n", func_name));
                }
                result
            }
            Expression::Index { object, index } => {
                let mut result = self.gen_expr_str(object);
                result.push_str(&self.gen_expr_str(index));
                result.push_str("    i32.load\n");
                result
            }
            Expression::MemberAccess { object, member } => {
                let mut result = self.gen_expr_str(object);
                result.push_str(&format!("    ;; member access: .{}\n", member));
                result
            }
            _ => String::new(),
        }
    }

    fn gen_binary_op(&mut self, left: &Expression, op: &BinaryOperator, right: &Expression) -> String {
        let mut result = self.gen_expr_str(left);
        result.push_str(&self.gen_expr_str(right));
        
        let op_str = match op {
            BinaryOperator::Add => "i32.add",
            BinaryOperator::Sub => "i32.sub",
            BinaryOperator::Mul => "i32.mul",
            BinaryOperator::Div => "i32.div_s",
            BinaryOperator::Mod => "i32.rem_s",
            BinaryOperator::Eq => "i32.eq",
            BinaryOperator::NotEq => "i32.ne",
            BinaryOperator::Less => "i32.lt_s",
            BinaryOperator::Greater => "i32.gt_s",
            BinaryOperator::LessEq => "i32.le_s",
            BinaryOperator::GreaterEq => "i32.ge_s",
            BinaryOperator::And => "i32.and",
            BinaryOperator::Or => "i32.or",
            BinaryOperator::NullCoalesce => "i32.or",
        };
        
        result.push_str(&format!("    {}\n", op_str));
        result
    }

    #[allow(dead_code)]
    pub fn finalize(&mut self) -> String {
        let mut final_output = "(module\n".to_string();
        for func in &self.functions {
            final_output.push_str(func);
        }
        final_output.push_str(")\n");
        final_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::lexer::Lexer;

    fn parse_and_generate(code: &str) -> String {
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap_or_default();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap_or_else(|_| Program { statements: vec![] });
        
        let mut codegen = WasmCodeGen::new();
        codegen.generate(&program)
    }

    #[test]
    fn test_wasm_module_creation() {
        let code = "let x = 42";
        let result = parse_and_generate(code);
        assert!(result.contains("(module"));
        assert!(result.contains(")"));
    }

    #[test]
    fn test_wasm_integer_const() {
        let mut codegen = WasmCodeGen::new();
        let expr = Expression::IntLiteral(42);
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.const 42"));
    }

    #[test]
    fn test_wasm_float_const() {
        let mut codegen = WasmCodeGen::new();
        let expr = Expression::FloatLiteral(3.14);
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("f32.const 3.14"));
    }

    #[test]
    fn test_wasm_bool_true() {
        let mut codegen = WasmCodeGen::new();
        let expr = Expression::BoolLiteral(true);
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.const 1"));
    }

    #[test]
    fn test_wasm_bool_false() {
        let mut codegen = WasmCodeGen::new();
        let expr = Expression::BoolLiteral(false);
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.const 0"));
    }

    #[test]
    fn test_wasm_addition() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(10));
        let right = Box::new(Expression::IntLiteral(20));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Add,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.const 10"));
        assert!(result.contains("i32.const 20"));
        assert!(result.contains("i32.add"));
    }

    #[test]
    fn test_wasm_subtraction() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(30));
        let right = Box::new(Expression::IntLiteral(10));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Sub,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.sub"));
    }

    #[test]
    fn test_wasm_multiplication() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(5));
        let right = Box::new(Expression::IntLiteral(6));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Mul,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.mul"));
    }

    #[test]
    fn test_wasm_division() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(20));
        let right = Box::new(Expression::IntLiteral(4));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Div,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.div_s"));
    }

    #[test]
    fn test_wasm_modulo() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(17));
        let right = Box::new(Expression::IntLiteral(5));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Mod,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.rem_s"));
    }

    #[test]
    fn test_wasm_equality() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(42));
        let right = Box::new(Expression::IntLiteral(42));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Eq,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.eq"));
    }

    #[test]
    fn test_wasm_not_equal() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(10));
        let right = Box::new(Expression::IntLiteral(20));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::NotEq,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.ne"));
    }

    #[test]
    fn test_wasm_less_than() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(10));
        let right = Box::new(Expression::IntLiteral(20));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Less,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.lt_s"));
    }

    #[test]
    fn test_wasm_greater_than() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(20));
        let right = Box::new(Expression::IntLiteral(10));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Greater,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.gt_s"));
    }

    #[test]
    fn test_wasm_less_equal() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(10));
        let right = Box::new(Expression::IntLiteral(20));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::LessEq,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.le_s"));
    }

    #[test]
    fn test_wasm_greater_equal() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(20));
        let right = Box::new(Expression::IntLiteral(10));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::GreaterEq,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.ge_s"));
    }

    #[test]
    fn test_wasm_logical_and() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(1));
        let right = Box::new(Expression::IntLiteral(1));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::And,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.and"));
    }

    #[test]
    fn test_wasm_logical_or() {
        let mut codegen = WasmCodeGen::new();
        let left = Box::new(Expression::IntLiteral(1));
        let right = Box::new(Expression::IntLiteral(0));
        let expr = Expression::BinaryOp {
            left,
            op: BinaryOperator::Or,
            right,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.or"));
    }

    #[test]
    fn test_wasm_unary_negation() {
        let mut codegen = WasmCodeGen::new();
        let operand = Box::new(Expression::IntLiteral(42));
        let expr = Expression::UnaryOp {
            op: crate::parser::ast::UnaryOperator::Neg,
            operand,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.const 42"));
        assert!(result.contains("i32.mul"));
    }

    #[test]
    fn test_wasm_unary_not() {
        let mut codegen = WasmCodeGen::new();
        let operand = Box::new(Expression::IntLiteral(1));
        let expr = Expression::UnaryOp {
            op: crate::parser::ast::UnaryOperator::Not,
            operand,
        };
        let result = codegen.gen_expr_str(&expr);
        assert!(result.contains("i32.eqz"));
    }
}
