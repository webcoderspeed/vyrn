use crate::parser::ast::{Statement, Expression, BinaryOperator, Program};
use std::collections::HashMap;

/// WebAssembly Text Format (WAT) code generator
pub struct WasmCodeGen {
    output: String,
    locals: HashMap<String, usize>,
    local_count: usize,
    functions: Vec<String>,
}

impl WasmCodeGen {
    pub fn new() -> Self {
        WasmCodeGen {
            output: String::new(),
            locals: HashMap::new(),
            local_count: 0,
            functions: Vec::new(),
        }
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
                format!("    ;; string literal: \"{}\"\n", s)
            }
            Expression::Identifier(name) => {
                if let Some(&idx) = self.locals.get(name) {
                    format!("    local.get {}\n", idx)
                } else {
                    format!("    ;; undefined variable: {}\n", name)
                }
            }
            Expression::BinaryOp { left, op, right } => {
                let mut result = String::new();
                result.push_str(&self.gen_expr_str(left));
                result.push_str(&self.gen_expr_str(right));
                
                let op_str = match op {
                    BinaryOperator::Add => "i32.add",
                    BinaryOperator::Sub => "i32.sub",
                    BinaryOperator::Mul => "i32.mul",
                    BinaryOperator::Div => "i32.div_s",
                    _ => "i32.add",
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
            _ => String::new(),
        }
    }

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
}
