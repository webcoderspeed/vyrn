/// C Code Generator for Vryn
/// Transpiles Vryn AST to C code with comprehensive support for language features

use crate::parser::ast::*;
use std::collections::{HashSet, HashMap};

/// C Code Generator structure
pub struct CCodeGen {
    indent_level: usize,
    output: String,
    header_includes: HashSet<String>,
    #[allow(dead_code)]
    defined_functions: HashSet<String>,
    variable_types: HashMap<String, String>,
    in_loop: bool,
}

impl CCodeGen {
    pub fn new() -> Self {
        CCodeGen {
            indent_level: 0,
            output: String::new(),
            header_includes: HashSet::new(),
            defined_functions: HashSet::new(),
            variable_types: HashMap::new(),
            in_loop: false,
        }
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn emit(&mut self, code: &str) {
        self.output.push_str(code);
    }

    fn emit_line(&mut self, code: &str) {
        self.emit(&self.indent());
        self.emit(code);
        self.emit("\n");
    }

    fn add_include(&mut self, header: &str) {
        self.header_includes.insert(header.to_string());
    }

    /// Generate complete C program from Vryn program
    pub fn generate(&mut self, program: &Program) -> String {
        // First pass: identify required headers
        self.scan_includes(program);

        // Emit headers
        self.emit_headers();

        // Second pass: generate code
        for statement in &program.statements {
            self.generate_statement(statement);
        }

        self.output.clone()
    }

    fn emit_headers(&mut self) {
        self.emit("#include <stdio.h>\n");
        self.emit("#include <stdlib.h>\n");
        self.emit("#include <string.h>\n");

        if self.header_includes.contains("math") {
            self.emit("#include <math.h>\n");
        }
        if self.header_includes.contains("stdbool") {
            self.emit("#include <stdbool.h>\n");
        }
        if self.header_includes.contains("time") {
            self.emit("#include <time.h>\n");
        }
        if self.header_includes.contains("ctype") {
            self.emit("#include <ctype.h>\n");
        }

        self.emit("\n");
    }

    fn scan_includes(&mut self, program: &Program) {
        for statement in &program.statements {
            self.scan_statement_includes(statement);
        }
    }

    fn scan_statement_includes(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Function { body, .. } => {
                for s in body {
                    self.scan_statement_includes(s);
                }
            }
            Statement::If { then_body, else_body, .. } => {
                for s in then_body {
                    self.scan_statement_includes(s);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.scan_statement_includes(s);
                    }
                }
            }
            Statement::While { body, .. } => {
                for s in body {
                    self.scan_statement_includes(s);
                }
            }
            Statement::For { body, .. } => {
                for s in body {
                    self.scan_statement_includes(s);
                }
            }
            Statement::Expression(expr) => {
                self.scan_expression_includes(expr);
            }
            Statement::Let { value, .. } => {
                self.scan_expression_includes(value);
            }
            Statement::Const { value, .. } => {
                self.scan_expression_includes(value);
            }
            _ => {}
        }
    }

    fn scan_expression_includes(&mut self, expr: &Expression) {
        match expr {
            Expression::Call { function, args } => {
                if let Expression::Identifier(name) = function.as_ref() {
                    match name.as_str() {
                        "sqrt" | "pow" | "floor" | "ceil" | "round" | 
                        "math_sin" | "math_cos" | "math_tan" | "math_log" | 
                        "math_exp" => {
                            self.add_include("math");
                        }
                        _ => {}
                    }
                }
                for arg in args {
                    self.scan_expression_includes(arg);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.scan_expression_includes(left);
                self.scan_expression_includes(right);
            }
            Expression::UnaryOp { operand, .. } => {
                self.scan_expression_includes(operand);
            }
            _ => {}
        }
    }

    /// Generate a single statement
    pub fn generate_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { name, value, .. } => {
                let type_str = self.infer_type_from_expr(value);
                self.variable_types.insert(name.clone(), type_str.clone());
                let value_str = self.generate_expression(value);
                self.emit_line(&format!("{} {} = {};", type_str, name, value_str));
            }
            Statement::Const { name, value } => {
                let type_str = self.infer_type_from_expr(value);
                self.variable_types.insert(name.clone(), type_str.clone());
                let value_str = self.generate_expression(value);
                self.emit_line(&format!("const {} {} = {};", type_str, name, value_str));
            }
            Statement::Function { name, params, return_type, body, is_async: _ } => {
                let ret_type = return_type.as_ref().map(|s| s.as_str()).unwrap_or("int");
                self.emit(&self.indent());
                self.emit(&format!("{} {}(", ret_type, name));

                for (i, param) in params.iter().enumerate() {
                    if i > 0 { self.emit(", "); }
                    self.emit(&format!("{} {}", param.type_name, param.name));
                    self.variable_types.insert(param.name.clone(), param.type_name.clone());
                }
                self.emit(") {\n");

                self.indent_level += 1;
                for s in body {
                    self.generate_statement(s);
                }
                self.indent_level -= 1;

                self.emit_line("}");
            }
            Statement::Expression(expr) => {
                let expr_str = self.generate_expression(expr);
                self.emit_line(&format!("{};", expr_str));
            }
            Statement::Return(expr_opt) => {
                match expr_opt {
                    Some(expr) => {
                        let expr_str = self.generate_expression(expr);
                        self.emit_line(&format!("return {};", expr_str));
                    }
                    None => self.emit_line("return;"),
                }
            }
            Statement::If { condition, then_body, else_body } => {
                let cond_str = self.generate_expression(condition);
                self.emit(&self.indent());
                self.emit(&format!("if ({}) {{\n", cond_str));
                self.indent_level += 1;
                for s in then_body {
                    self.generate_statement(s);
                }
                self.indent_level -= 1;

                if let Some(else_stmts) = else_body {
                    self.emit_line("} else {");
                    self.indent_level += 1;
                    for s in else_stmts {
                        self.generate_statement(s);
                    }
                    self.indent_level -= 1;
                    self.emit_line("}");
                } else {
                    self.emit_line("}");
                }
            }
            Statement::While { condition, body } => {
                let cond_str = self.generate_expression(condition);
                self.emit(&self.indent());
                self.emit(&format!("while ({}) {{\n", cond_str));
                
                self.indent_level += 1;
                let old_in_loop = self.in_loop;
                self.in_loop = true;
                
                for s in body {
                    self.generate_statement(s);
                }
                
                self.in_loop = old_in_loop;
                self.indent_level -= 1;
                self.emit_line("}");
            }
            Statement::For { var, iterable, body } => {
                // Generate a proper C for loop from Vryn for-in loop
                match iterable.as_ref() {
                    Expression::Range { start, end, inclusive } => {
                        let start_str = self.generate_expression(start);
                        let end_str = self.generate_expression(end);
                        let cond = if *inclusive { 
                            format!("{} <= {}", var, end_str)
                        } else {
                            format!("{} < {}", var, end_str)
                        };
                        
                        self.emit(&self.indent());
                        self.emit(&format!("for (int {} = {}; {}; {}++) {{\n", var, start_str, cond, var));
                        
                        self.indent_level += 1;
                        let old_in_loop = self.in_loop;
                        self.in_loop = true;
                        self.variable_types.insert(var.clone(), "int".to_string());
                        
                        for s in body {
                            self.generate_statement(s);
                        }
                        
                        self.in_loop = old_in_loop;
                        self.indent_level -= 1;
                        self.emit_line("}");
                    }
                    // For array iteration
                    Expression::Identifier(arr_name) => {
                        self.emit(&self.indent());
                        self.emit(&format!("for (int i = 0; i < strlen({}); i++) {{\n", arr_name));
                        self.emit(&self.indent());
                        self.emit(&format!("    char {} = {}[i];\n", var, arr_name));
                        
                        self.indent_level += 1;
                        let old_in_loop = self.in_loop;
                        self.in_loop = true;
                        self.variable_types.insert(var.clone(), "char".to_string());
                        
                        for s in body {
                            self.generate_statement(s);
                        }
                        
                        self.in_loop = old_in_loop;
                        self.indent_level -= 1;
                        self.emit_line("}");
                    }
                    _ => {
                        // Fallback for complex iterables
                        self.emit(&self.indent());
                        self.emit(&format!("for (int {} = 0; {} < 100; {}++) {{\n", var, var, var));
                        self.indent_level += 1;
                        let old_in_loop = self.in_loop;
                        self.in_loop = true;
                        
                        for s in body {
                            self.generate_statement(s);
                        }
                        
                        self.in_loop = old_in_loop;
                        self.indent_level -= 1;
                        self.emit_line("}");
                    }
                }
            }
            Statement::Break => {
                if self.in_loop {
                    self.emit_line("break;");
                }
            }
            Statement::Continue => {
                if self.in_loop {
                    self.emit_line("continue;");
                }
            }
            Statement::Struct { name, fields } => {
                self.emit(&self.indent());
                self.emit("typedef struct {\n");
                self.indent_level += 1;
                for field in fields {
                    self.emit_line(&format!("{} {};", field.type_name, field.name));
                }
                self.indent_level -= 1;
                self.emit_line(&format!("}} {};", name));
            }
            Statement::Enum { name, variants } => {
                self.emit(&self.indent());
                self.emit("typedef enum {\n");
                self.indent_level += 1;
                for (i, variant) in variants.iter().enumerate() {
                    let comma = if i < variants.len() - 1 { "," } else { "" };
                    self.emit_line(&format!("{}{}", variant.name, comma));
                }
                self.indent_level -= 1;
                self.emit_line(&format!("}} {};", name));
            }
            Statement::Import { path, .. } => {
                self.emit_line(&format!("// import {}", path));
            }
            _ => {
                self.emit_line("// unimplemented statement");
            }
        }
    }

    /// Generate C code for an expression
    pub fn generate_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::IntLiteral(n) => n.to_string(),
            Expression::FloatLiteral(f) => f.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s.escape_default()),
            Expression::BoolLiteral(b) => {
                if *b { "1" } else { "0" }.to_string()
            }
            Expression::Identifier(name) => name.clone(),
            Expression::BinaryOp { left, op, right } => {
                let left_str = self.generate_expression(left);
                let right_str = self.generate_expression(right);
                let op_str = self.generate_binary_op(op);
                format!("({} {} {})", left_str, op_str, right_str)
            }
            Expression::UnaryOp { op, operand } => {
                let operand_str = self.generate_expression(operand);
                match op {
                    UnaryOperator::Neg => format!("(-{})", operand_str),
                    UnaryOperator::Not => format!("(!{})", operand_str),
                }
            }
            Expression::Call { function, args } => {
                self.generate_function_call(function, args)
            }
            Expression::Array(elements) => {
                let elems_str = elements.iter()
                    .map(|e| self.generate_expression(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", elems_str)
            }
            Expression::Assign { target, value } => {
                let target_str = self.generate_expression(target);
                let value_str = self.generate_expression(value);
                format!("({} = {})", target_str, value_str)
            }
            Expression::MemberAccess { object, member } => {
                let obj_str = self.generate_expression(object);
                format!("({}.{})", obj_str, member)
            }
            Expression::Index { object, index } => {
                let obj_str = self.generate_expression(object);
                let idx_str = self.generate_expression(index);
                format!("({}[{}])", obj_str, idx_str)
            }
            Expression::Range { start, end, inclusive } => {
                let start_str = self.generate_expression(start);
                let end_str = self.generate_expression(end);
                if *inclusive {
                    format!("// range {}..={}", start_str, end_str)
                } else {
                    format!("// range {}..{}", start_str, end_str)
                }
            }
            Expression::StructInit { name, fields } => {
                let fields_str = fields.iter()
                    .map(|(k, v)| format!(".{} = {}", k, self.generate_expression(v)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("(struct {} {{ {} }})", name, fields_str)
            }
            Expression::Block(stmts) => {
                let mut result = String::from("(");
                for stmt in stmts {
                    if let Statement::Expression(e) = stmt {
                        result.push_str(&self.generate_expression(e));
                    }
                }
                result.push(')');
                result
            }
            _ => "0".to_string(),
        }
    }

    fn generate_function_call(&mut self, function: &Expression, args: &[Expression]) -> String {
        let func_name = match function {
            Expression::Identifier(name) => name.clone(),
            _ => self.generate_expression(function),
        };

        // Handle special built-in functions
        match func_name.as_str() {
            "println" => {
                if !args.is_empty() {
                    let arg_type = self.infer_expression_type(&args[0]);
                    let format_str = match arg_type.as_str() {
                        "double" => "%lf\\n",
                        "char*" => "%s\\n",
                        "int" | _ => "%d\\n",
                    };
                    let args_str = args.iter()
                        .map(|a| self.generate_expression(a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("printf(\"{}\", {})", format_str, args_str)
                } else {
                    "printf(\"\\n\")".to_string()
                }
            }
            "print" => {
                if !args.is_empty() {
                    let arg_type = self.infer_expression_type(&args[0]);
                    let format_str = match arg_type.as_str() {
                        "double" => "%lf",
                        "char*" => "%s",
                        "int" | _ => "%d",
                    };
                    let args_str = args.iter()
                        .map(|a| self.generate_expression(a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("printf(\"{}\", {})", format_str, args_str)
                } else {
                    "printf(\"\")".to_string()
                }
            }
            "len" | "str_len" => {
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("strlen({})", arg_str)
                } else {
                    "0".to_string()
                }
            }
            "str_upper" => {
                self.add_include("ctype");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("// str_upper({})", arg_str)
                } else {
                    "NULL".to_string()
                }
            }
            "str_lower" => {
                self.add_include("ctype");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("// str_lower({})", arg_str)
                } else {
                    "NULL".to_string()
                }
            }
            "str_trim" => {
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("// str_trim({})", arg_str)
                } else {
                    "NULL".to_string()
                }
            }
            "str_contains" => {
                if args.len() >= 2 {
                    let haystack = self.generate_expression(&args[0]);
                    let needle = self.generate_expression(&args[1]);
                    format!("(strstr({}, {}) != NULL)", haystack, needle)
                } else {
                    "0".to_string()
                }
            }
            "str_split" => {
                if args.len() >= 2 {
                    let str_arg = self.generate_expression(&args[0]);
                    let delim = self.generate_expression(&args[1]);
                    format!("// str_split({}, {})", str_arg, delim)
                } else {
                    "NULL".to_string()
                }
            }
            "arr_len" => {
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("strlen({})", arg_str)
                } else {
                    "0".to_string()
                }
            }
            "arr_reverse" => {
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("// arr_reverse({})", arg_str)
                } else {
                    "NULL".to_string()
                }
            }
            "arr_contains" => {
                if args.len() >= 2 {
                    let arr = self.generate_expression(&args[0]);
                    let val = self.generate_expression(&args[1]);
                    format!("// arr_contains({}, {})", arr, val)
                } else {
                    "0".to_string()
                }
            }
            "arr_sort" => {
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    self.add_include("stdlib.h");
                    format!("// arr_sort({})", arg_str)
                } else {
                    "NULL".to_string()
                }
            }
            "sqrt" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("sqrt({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            "abs" => {
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("abs({})", arg_str)
                } else {
                    "0".to_string()
                }
            }
            "min" => {
                if args.len() >= 2 {
                    let a = self.generate_expression(&args[0]);
                    let b = self.generate_expression(&args[1]);
                    format!("(({} < {}) ? {} : {})", a, b, a, b)
                } else {
                    "0".to_string()
                }
            }
            "max" => {
                if args.len() >= 2 {
                    let a = self.generate_expression(&args[0]);
                    let b = self.generate_expression(&args[1]);
                    format!("(({} > {}) ? {} : {})", a, b, a, b)
                } else {
                    "0".to_string()
                }
            }
            "floor" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("floor({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            "ceil" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("ceil({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            "round" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("round({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            "pow" => {
                self.add_include("math");
                if args.len() >= 2 {
                    let base = self.generate_expression(&args[0]);
                    let exp = self.generate_expression(&args[1]);
                    format!("pow({}, {})", base, exp)
                } else {
                    "0.0".to_string()
                }
            }
            "math_sin" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("sin({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            "math_cos" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("cos({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            "math_tan" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("tan({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            "math_log" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("log({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            "math_exp" => {
                self.add_include("math");
                if !args.is_empty() {
                    let arg_str = self.generate_expression(&args[0]);
                    format!("exp({})", arg_str)
                } else {
                    "0.0".to_string()
                }
            }
            _ => {
                // Regular function call
                let args_str = args.iter()
                    .map(|a| self.generate_expression(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", func_name, args_str)
            }
        }
    }

    fn generate_binary_op(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mul => "*",
            BinaryOperator::Div => "/",
            BinaryOperator::Mod => "%",
            BinaryOperator::Eq => "==",
            BinaryOperator::NotEq => "!=",
            BinaryOperator::Less => "<",
            BinaryOperator::Greater => ">",
            BinaryOperator::LessEq => "<=",
            BinaryOperator::GreaterEq => ">=",
            BinaryOperator::And => "&&",
            BinaryOperator::Or => "||",
            BinaryOperator::NullCoalesce => "??",
        }
    }

    /// Infer type from an expression, using variable types when available
    fn infer_expression_type(&self, expr: &Expression) -> String {
        match expr {
            Expression::IntLiteral(_) => "int".to_string(),
            Expression::FloatLiteral(_) => "double".to_string(),
            Expression::StringLiteral(_) => "char*".to_string(),
            Expression::BoolLiteral(_) => "int".to_string(),
            Expression::Array(_) => "int*".to_string(),
            Expression::Identifier(name) => {
                // Check if we know the variable type
                self.variable_types.get(name).cloned().unwrap_or_else(|| "int".to_string())
            }
            Expression::BinaryOp { .. } => {
                // Binary ops usually return the left operand type
                "int".to_string()
            }
            _ => "int".to_string(),
        }
    }

    fn infer_type_from_expr(&self, expr: &Expression) -> String {
        match expr {
            Expression::IntLiteral(_) => "int".to_string(),
            Expression::FloatLiteral(_) => "double".to_string(),
            Expression::StringLiteral(_) => "char*".to_string(),
            Expression::BoolLiteral(_) => "int".to_string(),
            Expression::Array(_) => "int*".to_string(),
            Expression::Identifier(_) => "int".to_string(),
            _ => "int".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::lexer::Lexer;

    fn parse_vryn(code: &str) -> Program {
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap_or_default();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap_or_else(|_| Program { statements: vec![] })
    }

    fn generate_c(code: &str) -> String {
        let program = parse_vryn(code);
        let mut codegen = CCodeGen::new();
        codegen.generate(&program)
    }

    #[test]
    fn test_simple_main_function() {
        let code = "fun main() { }";
        let c_code = generate_c(code);
        assert!(c_code.contains("int main()"));
        assert!(c_code.contains("#include <stdio.h>"));
    }

    #[test]
    fn test_integer_literal() {
        let code = "fun main() { let x = 42 }";
        let c_code = generate_c(code);
        assert!(c_code.contains("int x = 42"));
    }

    #[test]
    fn test_floating_point_literal() {
        let code = "fun main() { let x = 3.14 }";
        let c_code = generate_c(code);
        assert!(c_code.contains("double x = 3.14"));
    }

    #[test]
    fn test_string_literal() {
        let code = "fun main() { let msg = \"hello\" }";
        let c_code = generate_c(code);
        assert!(c_code.contains("char* msg = \"hello\""));
    }

    #[test]
    fn test_boolean_literal() {
        let code = "fun main() { let flag = true }";
        let c_code = generate_c(code);
        assert!(c_code.contains("int flag = 1"));
    }

    #[test]
    fn test_binary_operations() {
        let code = "fun main() { let sum = 5 + 3 }";
        let c_code = generate_c(code);
        assert!(c_code.contains("+"));
    }

    #[test]
    fn test_function_with_params() {
        let code = "fun add(a: int, b: int) -> int { return a + b }";
        let c_code = generate_c(code);
        assert!(c_code.contains("int add(int a, int b)"));
        assert!(c_code.contains("return"));
    }

    #[test]
    fn test_if_statement() {
        let code = "fun main() { if true { let x = 1 } }";
        let c_code = generate_c(code);
        assert!(c_code.contains("if ("));
    }

    #[test]
    fn test_while_loop() {
        let code = "fun main() { while true { let x = 1 } }";
        let c_code = generate_c(code);
        assert!(c_code.contains("while ("));
    }

    #[test]
    fn test_const_declaration() {
        let code = "const PI = 3.14";
        let c_code = generate_c(code);
        assert!(c_code.contains("const"));
    }

    #[test]
    fn test_for_in_loop() {
        let code = "fun main() { for i in 0..5 { println(i) } }";
        let c_code = generate_c(code);
        assert!(c_code.contains("for (int i = 0"));
        assert!(c_code.contains("i < 5"));
        assert!(c_code.contains("i++)"));
    }

    #[test]
    fn test_for_in_loop_inclusive() {
        let code = "fun main() { for i in 0..=10 { } }";
        let c_code = generate_c(code);
        assert!(c_code.contains("i <= 10"));
    }

    #[test]
    fn test_string_operations() {
        let code = "fun main() { let has = str_contains(\"hello\", \"ll\") }";
        let c_code = generate_c(code);
        assert!(c_code.contains("strstr"));
    }

    #[test]
    fn test_math_functions() {
        let code = "fun main() { let root = sqrt(16.0) }";
        let c_code = generate_c(code);
        assert!(c_code.contains("sqrt"));
        assert!(c_code.contains("#include <math.h>"));
    }

    #[test]
    fn test_array_operations() {
        let code = "fun main() { let len = arr_len(\"hello\") }";
        let c_code = generate_c(code);
        assert!(c_code.contains("strlen"));
    }

    #[test]
    fn test_min_max_functions() {
        let code = "fun main() { let m = min(5, 10) }";
        let c_code = generate_c(code);
        assert!(c_code.contains("?"));
        assert!(c_code.contains(":"));
    }

    #[test]
    fn test_struct_declaration() {
        let code = "struct Point { x: int, y: int }";
        let c_code = generate_c(code);
        assert!(c_code.contains("typedef struct"));
        assert!(c_code.contains("Point"));
    }

    #[test]
    fn test_enum_declaration() {
        let code = "enum Color { Red, Green, Blue }";
        let c_code = generate_c(code);
        assert!(c_code.contains("typedef enum"));
        assert!(c_code.contains("Color"));
    }

    #[test]
    fn test_includes_are_present() {
        let code = "fun main() { }";
        let c_code = generate_c(code);
        assert!(c_code.contains("#include <stdio.h>"));
        assert!(c_code.contains("#include <stdlib.h>"));
        assert!(c_code.contains("#include <string.h>"));
    }

    #[test]
    fn test_break_in_loop() {
        let code = "fun main() { while true { break } }";
        let c_code = generate_c(code);
        assert!(c_code.contains("break;"));
    }

    #[test]
    fn test_continue_in_loop() {
        let code = "fun main() { while true { continue } }";
        let c_code = generate_c(code);
        assert!(c_code.contains("continue;"));
    }

    #[test]
    fn test_nested_if_statements() {
        let code = "fun main() { if true { if false { let x = 1 } } }";
        let c_code = generate_c(code);
        assert!(c_code.contains("if ("));
    }
}
