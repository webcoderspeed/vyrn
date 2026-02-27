use crate::parser::ast::*;

/// A code formatter for Vryn that takes a parsed AST and outputs formatted source code
pub struct Formatter;

impl Formatter {
    /// Create a new formatter
    pub fn new() -> Self {
        Formatter
    }

    /// Format a program (the full AST)
    pub fn format_program(&mut self, program: &Program) -> String {
        Self::format_program_impl(program, 0)
    }

    /// Format a program with indentation level
    fn format_program_impl(program: &Program, indent_level: usize) -> String {
        let mut output = String::new();

        for (i, statement) in program.statements.iter().enumerate() {
            output.push_str(&Self::format_statement_impl(statement, indent_level));

            // Add blank line between top-level declarations
            if i < program.statements.len() - 1 {
                // Check if current statement is a declaration that should have spacing
                if matches!(
                    statement,
                    Statement::Function { .. }
                        | Statement::Struct { .. }
                        | Statement::Enum { .. }
                        | Statement::Trait { .. }
                        | Statement::Impl { .. }
                        | Statement::Import { .. }
                ) {
                    output.push('\n');
                }
            }
        }

        output
    }

    /// Get indentation string
    fn indent(level: usize) -> String {
        " ".repeat(level * 4)
    }

    /// Format a single statement
    fn format_statement_impl(statement: &Statement, indent_level: usize) -> String {
        match statement {
            Statement::Let {
                name,
                mutable,
                type_ann,
                value,
            } => {
                let mut result = Self::indent(indent_level);
                result.push_str("let ");
                if *mutable {
                    result.push_str("mut ");
                }
                result.push_str(name);
                if let Some(ann) = type_ann {
                    result.push_str(": ");
                    result.push_str(ann);
                }
                result.push_str(" = ");
                result.push_str(&Self::format_expression_impl(value, indent_level));
                result.push('\n');
                result
            }
            Statement::Const { name, value } => {
                let mut result = Self::indent(indent_level);
                result.push_str("const ");
                result.push_str(name);
                result.push_str(" = ");
                result.push_str(&Self::format_expression_impl(value, indent_level));
                result.push('\n');
                result
            }
            Statement::Function {
                name,
                params,
                return_type,
                body,
                is_async,
            } => {
                let mut result = Self::indent(indent_level);
                if *is_async {
                    result.push_str("async ");
                }
                result.push_str("fun ");
                result.push_str(name);
                result.push('(');

                for (i, param) in params.iter().enumerate() {
                    result.push_str(&param.name);
                    result.push_str(": ");
                    result.push_str(&param.type_name);
                    if i < params.len() - 1 {
                        result.push_str(", ");
                    }
                }

                result.push(')');

                if let Some(ret_type) = return_type {
                    result.push_str(" -> ");
                    result.push_str(ret_type);
                }

                result.push_str(" {\n");

                for stmt in body {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }

                result.push_str(&Self::indent(indent_level));
                result.push_str("}\n");
                result
            }
            Statement::Struct { name, fields } => {
                let mut result = Self::indent(indent_level);
                result.push_str("struct ");
                result.push_str(name);
                result.push_str(" {\n");

                for field in fields {
                    result.push_str(&Self::indent(indent_level + 1));
                    result.push_str(&field.name);
                    result.push_str(": ");
                    result.push_str(&field.type_name);
                    result.push(',');
                    result.push('\n');
                }

                result.push_str(&Self::indent(indent_level));
                result.push_str("}\n");
                result
            }
            Statement::Enum { name, variants } => {
                let mut result = Self::indent(indent_level);
                result.push_str("enum ");
                result.push_str(name);
                result.push_str(" {\n");

                for variant in variants {
                    result.push_str(&Self::indent(indent_level + 1));
                    result.push_str(&variant.name);
                    if !variant.fields.is_empty() {
                        result.push('(');
                        for (i, field) in variant.fields.iter().enumerate() {
                            result.push_str(field);
                            if i < variant.fields.len() - 1 {
                                result.push_str(", ");
                            }
                        }
                        result.push(')');
                    }
                    result.push(',');
                    result.push('\n');
                }

                result.push_str(&Self::indent(indent_level));
                result.push_str("}\n");
                result
            }
            Statement::Trait { name, methods } => {
                let mut result = Self::indent(indent_level);
                result.push_str("trait ");
                result.push_str(name);
                result.push_str(" {\n");

                for method in methods {
                    result.push_str(&Self::indent(indent_level + 1));
                    result.push_str("fun ");
                    result.push_str(&method.name);
                    result.push('(');

                    for (i, param) in method.params.iter().enumerate() {
                        result.push_str(&param.name);
                        result.push_str(": ");
                        result.push_str(&param.type_name);
                        if i < method.params.len() - 1 {
                            result.push_str(", ");
                        }
                    }

                    result.push(')');

                    if let Some(ret_type) = &method.return_type {
                        result.push_str(" -> ");
                        result.push_str(ret_type);
                    }

                    result.push_str(";\n");
                }

                result.push_str(&Self::indent(indent_level));
                result.push_str("}\n");
                result
            }
            Statement::Impl {
                trait_name,
                type_name,
                methods,
            } => {
                let mut result = Self::indent(indent_level);
                result.push_str("impl ");
                if let Some(tname) = trait_name {
                    result.push_str(tname);
                    result.push_str(" for ");
                }
                result.push_str(type_name);
                result.push_str(" {\n");

                for method in methods {
                    result.push_str(&Self::indent(indent_level + 1));
                    result.push_str("fun ");
                    result.push_str(&method.name);
                    result.push('(');

                    for (i, param) in method.params.iter().enumerate() {
                        result.push_str(&param.name);
                        result.push_str(": ");
                        result.push_str(&param.type_name);
                        if i < method.params.len() - 1 {
                            result.push_str(", ");
                        }
                    }

                    result.push(')');

                    if let Some(ret_type) = &method.return_type {
                        result.push_str(" -> ");
                        result.push_str(ret_type);
                    }

                    result.push_str(" {\n");

                    for stmt in &method.body {
                        result.push_str(&Self::format_statement_impl(stmt, indent_level + 2));
                    }

                    result.push_str(&Self::indent(indent_level + 1));
                    result.push_str("}\n");
                }

                result.push_str(&Self::indent(indent_level));
                result.push_str("}\n");
                result
            }
            Statement::Import { path, alias } => {
                let mut result = Self::indent(indent_level);
                result.push_str("import ");
                result.push_str(path);
                if let Some(a) = alias {
                    result.push_str(" as ");
                    result.push_str(a);
                }
                result.push('\n');
                result
            }
            Statement::Expression(expr) => {
                let mut result = Self::indent(indent_level);
                result.push_str(&Self::format_expression_impl(expr, indent_level));
                result.push('\n');
                result
            }
            Statement::Return(expr) => {
                let mut result = Self::indent(indent_level);
                result.push_str("return");
                if let Some(e) = expr {
                    result.push(' ');
                    result.push_str(&Self::format_expression_impl(e, indent_level));
                }
                result.push('\n');
                result
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let mut result = Self::indent(indent_level);
                result.push_str("if ");
                result.push_str(&Self::format_expression_impl(condition, indent_level));
                result.push_str(" {\n");

                for stmt in then_body {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }

                if let Some(else_stmts) = else_body {
                    result.push_str(&Self::indent(indent_level));
                    result.push_str("} else {\n");

                    for stmt in else_stmts {
                        result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                    }

                    result.push_str(&Self::indent(indent_level));
                    result.push_str("}\n");
                } else {
                    result.push_str(&Self::indent(indent_level));
                    result.push_str("}\n");
                }

                result
            }
            Statement::IfLet {
                pattern,
                expr,
                then_body,
                else_body,
            } => {
                let mut result = Self::indent(indent_level);
                result.push_str("if let ");
                result.push_str(&Self::format_pattern_impl(pattern, indent_level));
                result.push_str(" = ");
                result.push_str(&Self::format_expression_impl(expr, indent_level));
                result.push_str(" {\n");

                for stmt in then_body {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }

                if let Some(else_stmts) = else_body {
                    result.push_str(&Self::indent(indent_level));
                    result.push_str("} else {\n");

                    for stmt in else_stmts {
                        result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                    }

                    result.push_str(&Self::indent(indent_level));
                    result.push_str("}\n");
                } else {
                    result.push_str(&Self::indent(indent_level));
                    result.push_str("}\n");
                }

                result
            }
            Statement::While { condition, body } => {
                let mut result = Self::indent(indent_level);
                result.push_str("while ");
                result.push_str(&Self::format_expression_impl(condition, indent_level));
                result.push_str(" {\n");

                for stmt in body {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }

                result.push_str(&Self::indent(indent_level));
                result.push_str("}\n");
                result
            }
            Statement::WhileLet {
                pattern,
                expr,
                body,
            } => {
                let mut result = Self::indent(indent_level);
                result.push_str("while let ");
                result.push_str(&Self::format_pattern_impl(pattern, indent_level));
                result.push_str(" = ");
                result.push_str(&Self::format_expression_impl(expr, indent_level));
                result.push_str(" {\n");

                for stmt in body {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }

                result.push_str(&Self::indent(indent_level));
                result.push_str("}\n");
                result
            }
            Statement::For {
                var,
                iterable,
                body,
            } => {
                let mut result = Self::indent(indent_level);
                result.push_str("for ");
                result.push_str(var);
                result.push_str(" in ");
                result.push_str(&Self::format_expression_impl(iterable, indent_level));
                result.push_str(" {\n");

                for stmt in body {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }

                result.push_str(&Self::indent(indent_level));
                result.push_str("}\n");
                result
            }
            Statement::Break => {
                let mut result = Self::indent(indent_level);
                result.push_str("break\n");
                result
            }
            Statement::Continue => {
                let mut result = Self::indent(indent_level);
                result.push_str("continue\n");
                result
            }
        }
    }

    /// Format an expression
    fn format_expression_impl(expr: &Expression, indent_level: usize) -> String {
        match expr {
            Expression::IntLiteral(n) => n.to_string(),
            Expression::FloatLiteral(f) => f.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s),
            Expression::BoolLiteral(b) => b.to_string(),
            Expression::Identifier(name) => name.clone(),
            Expression::BinaryOp { left, op, right } => {
                let op_str = match op {
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
                };
                format!(
                    "{} {} {}",
                    Self::format_expression_impl(left, indent_level),
                    op_str,
                    Self::format_expression_impl(right, indent_level)
                )
            }
            Expression::UnaryOp { op, operand } => {
                let op_str = match op {
                    UnaryOperator::Neg => "-",
                    UnaryOperator::Not => "!",
                };
                format!("{}{}", op_str, Self::format_expression_impl(operand, indent_level))
            }
            Expression::Call { function, args } => {
                let mut result = Self::format_expression_impl(function, indent_level);
                result.push('(');
                for (i, arg) in args.iter().enumerate() {
                    result.push_str(&Self::format_expression_impl(arg, indent_level));
                    if i < args.len() - 1 {
                        result.push_str(", ");
                    }
                }
                result.push(')');
                result
            }
            Expression::MemberAccess { object, member } => {
                format!("{}.{}", Self::format_expression_impl(object, indent_level), member)
            }
            Expression::Index { object, index } => {
                format!(
                    "{}[{}]",
                    Self::format_expression_impl(object, indent_level),
                    Self::format_expression_impl(index, indent_level)
                )
            }
            Expression::Assign { target, value } => {
                format!(
                    "{} = {}",
                    Self::format_expression_impl(target, indent_level),
                    Self::format_expression_impl(value, indent_level)
                )
            }
            Expression::Array(items) => {
                let mut result = String::from("[");
                for (i, item) in items.iter().enumerate() {
                    result.push_str(&Self::format_expression_impl(item, indent_level));
                    if i < items.len() - 1 {
                        result.push_str(", ");
                    }
                }
                result.push(']');
                result
            }
            Expression::Pipe { left, right } => {
                format!(
                    "{} |> {}",
                    Self::format_expression_impl(left, indent_level),
                    Self::format_expression_impl(right, indent_level)
                )
            }
            Expression::Range {
                start,
                end,
                inclusive,
            } => {
                let op = if *inclusive { "..=" } else { ".." };
                format!(
                    "{}{}{}",
                    Self::format_expression_impl(start, indent_level),
                    op,
                    Self::format_expression_impl(end, indent_level)
                )
            }
            Expression::Match { value, arms } => {
                let mut result = format!("match {} {{\n", Self::format_expression_impl(value, indent_level));
                for arm in arms {
                    result.push_str(&Self::indent(indent_level + 1));
                    result.push_str(&Self::format_pattern_impl(&arm.pattern, indent_level + 1));
                    result.push_str(" => ");
                    result.push_str(&Self::format_expression_impl(&arm.body, indent_level + 1));
                    result.push_str(",\n");
                }
                result.push_str(&Self::indent(indent_level));
                result.push('}');
                result
            }
            Expression::Block(stmts) => {
                let mut result = String::from("{\n");
                for stmt in stmts {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }
                result.push_str(&Self::indent(indent_level));
                result.push('}');
                result
            }
            Expression::StructInit { name, fields } => {
                let mut result = format!("{} {{\n", name);
                for (i, (field_name, value)) in fields.iter().enumerate() {
                    result.push_str(&Self::indent(indent_level + 1));
                    result.push_str(field_name);
                    result.push_str(": ");
                    result.push_str(&Self::format_expression_impl(value, indent_level + 1));
                    if i < fields.len() - 1 {
                        result.push(',');
                    }
                    result.push('\n');
                }
                result.push_str(&Self::indent(indent_level));
                result.push('}');
                result
            }
            Expression::Lambda { params, body } => {
                let mut result = String::from("|");
                for (i, param) in params.iter().enumerate() {
                    result.push_str(param);
                    if i < params.len() - 1 {
                        result.push_str(", ");
                    }
                }
                result.push_str("| ");
                result.push_str(&Self::format_expression_impl(body, indent_level));
                result
            }
            Expression::TryCatch {
                try_body,
                catch_var,
                catch_body,
            } => {
                let mut result = String::from("try {\n");
                for stmt in try_body {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }
                result.push_str(&Self::indent(indent_level));
                result.push_str("} catch ");
                result.push_str(catch_var);
                result.push_str(" {\n");
                for stmt in catch_body {
                    result.push_str(&Self::format_statement_impl(stmt, indent_level + 1));
                }
                result.push_str(&Self::indent(indent_level));
                result.push('}');
                result
            }
            Expression::QuestionMark { expr } => {
                format!("{}?", Self::format_expression_impl(expr, indent_level))
            }
            Expression::MethodCall {
                object,
                method,
                args,
            } => {
                let mut result = format!("{}.{}(", Self::format_expression_impl(object, indent_level), method);
                for (i, arg) in args.iter().enumerate() {
                    result.push_str(&Self::format_expression_impl(arg, indent_level));
                    if i < args.len() - 1 {
                        result.push_str(", ");
                    }
                }
                result.push(')');
                result
            }
            Expression::Self_ => "self".to_string(),
            Expression::Await { expr } => {
                format!("await {}", Self::format_expression_impl(expr, indent_level))
            }
            Expression::Spawn { body } => {
                format!("spawn {}", Self::format_expression_impl(body, indent_level))
            }
        }
    }

    /// Format a pattern
    fn format_pattern_impl(pattern: &Pattern, indent_level: usize) -> String {
        match pattern {
            Pattern::Literal(expr) => Self::format_expression_impl(expr, indent_level),
            Pattern::Identifier(name) => name.clone(),
            Pattern::Wildcard => "_".to_string(),
            Pattern::EnumVariant { name, fields } => {
                let mut result = name.clone();
                if !fields.is_empty() {
                    result.push('(');
                    for (i, field) in fields.iter().enumerate() {
                        result.push_str(&Self::format_pattern_impl(field, indent_level));
                        if i < fields.len() - 1 {
                            result.push_str(", ");
                        }
                    }
                    result.push(')');
                }
                result
            }
            Pattern::Tuple(patterns) => {
                let mut result = String::from("(");
                for (i, p) in patterns.iter().enumerate() {
                    result.push_str(&Self::format_pattern_impl(p, indent_level));
                    if i < patterns.len() - 1 {
                        result.push_str(", ");
                    }
                }
                result.push(')');
                result
            }
            Pattern::Struct { name, fields } => {
                let mut result = format!("{} {{\n", name);
                for (i, (field_name, pat)) in fields.iter().enumerate() {
                    result.push_str(&Self::indent(indent_level + 1));
                    result.push_str(field_name);
                    result.push_str(": ");
                    result.push_str(&Self::format_pattern_impl(pat, indent_level + 1));
                    if i < fields.len() - 1 {
                        result.push(',');
                    }
                    result.push('\n');
                }
                result.push_str(&Self::indent(indent_level));
                result.push('}');
                result
            }
            Pattern::Range {
                start,
                end,
                inclusive,
            } => {
                let op = if *inclusive { "..=" } else { ".." };
                format!(
                    "{}{}{}",
                    Self::format_expression_impl(start, indent_level),
                    op,
                    Self::format_expression_impl(end, indent_level)
                )
            }
            Pattern::Or(patterns) => {
                let mut result = String::new();
                for (i, p) in patterns.iter().enumerate() {
                    result.push_str(&Self::format_pattern_impl(p, indent_level));
                    if i < patterns.len() - 1 {
                        result.push_str(" | ");
                    }
                }
                result
            }
            Pattern::Guard {
                pattern,
                condition,
            } => {
                format!(
                    "{} if {}",
                    Self::format_pattern_impl(pattern, indent_level),
                    Self::format_expression_impl(condition, indent_level)
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_let() {
        let program = Program {
            statements: vec![Statement::Let {
                name: "x".to_string(),
                mutable: false,
                type_ann: None,
                value: Expression::IntLiteral(42),
            }],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert_eq!(output, "let x = 42\n");
    }

    #[test]
    fn test_format_let_with_type() {
        let program = Program {
            statements: vec![Statement::Let {
                name: "x".to_string(),
                mutable: false,
                type_ann: Some("i32".to_string()),
                value: Expression::IntLiteral(42),
            }],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert_eq!(output, "let x: i32 = 42\n");
    }

    #[test]
    fn test_format_mutable_var() {
        let program = Program {
            statements: vec![Statement::Let {
                name: "x".to_string(),
                mutable: true,
                type_ann: None,
                value: Expression::IntLiteral(0),
            }],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert_eq!(output, "let mut x = 0\n");
    }

    #[test]
    fn test_format_function() {
        let program = Program {
            statements: vec![Statement::Function {
                name: "add".to_string(),
                params: vec![
                    Param {
                        name: "a".to_string(),
                        type_name: "i32".to_string(),
                    },
                    Param {
                        name: "b".to_string(),
                        type_name: "i32".to_string(),
                    },
                ],
                return_type: Some("i32".to_string()),
                body: vec![Statement::Return(Some(Expression::BinaryOp {
                    left: Box::new(Expression::Identifier("a".to_string())),
                    op: BinaryOperator::Add,
                    right: Box::new(Expression::Identifier("b".to_string())),
                }))],
                is_async: false,
            }],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert!(output.contains("fun add(a: i32, b: i32) -> i32 {"));
        assert!(output.contains("return a + b"));
    }

    #[test]
    fn test_format_if_statement() {
        let program = Program {
            statements: vec![Statement::If {
                condition: Box::new(Expression::BoolLiteral(true)),
                then_body: vec![Statement::Expression(Expression::IntLiteral(1))],
                else_body: None,
            }],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert!(output.contains("if true {"));
        assert!(output.contains("1\n"));
    }

    #[test]
    fn test_format_struct() {
        let program = Program {
            statements: vec![Statement::Struct {
                name: "Point".to_string(),
                fields: vec![
                    Field {
                        name: "x".to_string(),
                        type_name: "i32".to_string(),
                    },
                    Field {
                        name: "y".to_string(),
                        type_name: "i32".to_string(),
                    },
                ],
            }],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert!(output.contains("struct Point {"));
        assert!(output.contains("x: i32,"));
        assert!(output.contains("y: i32,"));
    }

    #[test]
    fn test_format_array_expression() {
        let program = Program {
            statements: vec![Statement::Expression(Expression::Array(vec![
                Expression::IntLiteral(1),
                Expression::IntLiteral(2),
                Expression::IntLiteral(3),
            ]))],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert_eq!(output, "[1, 2, 3]\n");
    }

    #[test]
    fn test_format_binary_op() {
        let program = Program {
            statements: vec![Statement::Expression(Expression::BinaryOp {
                left: Box::new(Expression::IntLiteral(5)),
                op: BinaryOperator::Add,
                right: Box::new(Expression::IntLiteral(3)),
            })],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert_eq!(output, "5 + 3\n");
    }

    #[test]
    fn test_format_function_call() {
        let program = Program {
            statements: vec![Statement::Expression(Expression::Call {
                function: Box::new(Expression::Identifier("print".to_string())),
                args: vec![Expression::StringLiteral("hello".to_string())],
            })],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert_eq!(output, "print(\"hello\")\n");
    }

    #[test]
    fn test_format_for_loop() {
        let program = Program {
            statements: vec![Statement::For {
                var: "i".to_string(),
                iterable: Box::new(Expression::Range {
                    start: Box::new(Expression::IntLiteral(0)),
                    end: Box::new(Expression::IntLiteral(10)),
                    inclusive: false,
                }),
                body: vec![Statement::Expression(Expression::Identifier("i".to_string()))],
            }],
        };

        let mut formatter = Formatter::new();
        let output = formatter.format_program(&program);
        assert!(output.contains("for i in 0..10 {"));
    }
}
