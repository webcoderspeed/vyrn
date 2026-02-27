/// Vryn Interpreter — Tree-walking interpreter for Phase 1 MVP
/// (Later will be replaced with C/LLVM code generation)

use std::collections::HashMap;
use crate::parser::ast::*;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Array(Vec<Value>),
    /// Struct instance: Point { x: 1, y: 2 }
    Struct {
        name: String,
        fields: HashMap<String, Value>,
    },
    None,
    /// Function value for first-class functions
    Function {
        name: String,
        params: Vec<Param>,
        body: Vec<Statement>,
    },
    /// Enum Type Definition (e.g. enum Color)
    EnumType {
        name: String,
        variants: Vec<String>,
    },
    /// Enum Variant Instance (e.g. Color::Red or Option::Some(5))
    Variant {
        enum_name: String,
        variant: String,
        values: Vec<Value>,
    },
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Struct { name, fields } => {
                write!(f, "{} {{ ", name)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            Value::None => write!(f, "None"),
            Value::Function { name, .. } => write!(f, "<fn {}>", name),
            Value::EnumType { name, .. } => write!(f, "<enum {}>", name),
            Value::Variant { enum_name, variant, values } => {
                if values.is_empty() {
                    write!(f, "{}::{}", enum_name, variant)
                } else {
                    write!(f, "{}::{}(", enum_name, variant)?;
                    for (i, v) in values.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", v)?;
                    }
                    write!(f, ")")
                }
            }
        }
    }
}

/// The runtime environment — holds variables in nested scopes
#[derive(Debug, Clone)]
pub struct Environment {
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val);
            }
        }
        None
    }

    pub fn set(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        Err(format!("Undefined variable '{}'", name))
    }
}

/// Signal for control flow
enum Signal {
    None,
    Return(Value),
    Break,
    Continue,
}

/// The Vryn Interpreter
pub struct Interpreter {
    env: Environment,
    output: Vec<String>,  // captured output for testing
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: Environment::new(),
            output: Vec::new(),
        }
    }

    pub fn get_output(&self) -> &Vec<String> {
        &self.output
    }

    /// Run a complete Vryn program
    pub fn run(&mut self, program: &Program) -> Result<(), String> {
        for stmt in &program.statements {
            match self.exec_statement(stmt)? {
                Signal::Return(_) => break,
                _ => {}
            }
        }
        Ok(())
    }

    /// Execute a single statement
    fn exec_statement(&mut self, stmt: &Statement) -> Result<Signal, String> {
        match stmt {
            Statement::Let { name, value, .. } => {
                let val = self.eval_expression(value)?;
                self.env.define(name, val);
                Ok(Signal::None)
            }

            Statement::Function { name, params, body, .. } => {
                let func = Value::Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                };
                self.env.define(name, func);
                Ok(Signal::None)
            }

            Statement::Expression(expr) => {
                self.eval_expression(expr)?;
                Ok(Signal::None)
            }

            Statement::Return(expr) => {
                let val = if let Some(e) = expr {
                    self.eval_expression(e)?
                } else {
                    Value::None
                };
                Ok(Signal::Return(val))
            }

            Statement::If { condition, then_body, else_body } => {
                let cond = self.eval_expression(condition)?;
                if self.is_truthy(&cond) {
                    self.env.push_scope();
                    for s in then_body {
                        let sig = self.exec_statement(s)?;
                        match sig {
                            Signal::None => {}
                            other => { self.env.pop_scope(); return Ok(other); }
                        }
                    }
                    self.env.pop_scope();
                } else if let Some(else_stmts) = else_body {
                    self.env.push_scope();
                    for s in else_stmts {
                        let sig = self.exec_statement(s)?;
                        match sig {
                            Signal::None => {}
                            other => { self.env.pop_scope(); return Ok(other); }
                        }
                    }
                    self.env.pop_scope();
                }
                Ok(Signal::None)
            }

            Statement::While { condition, body } => {
                loop {
                    let cond = self.eval_expression(condition)?;
                    if !self.is_truthy(&cond) { break; }

                    self.env.push_scope();
                    let mut should_break = false;
                    for s in body {
                        match self.exec_statement(s)? {
                            Signal::Break => { should_break = true; break; }
                            Signal::Continue => break,
                            Signal::Return(v) => { self.env.pop_scope(); return Ok(Signal::Return(v)); }
                            Signal::None => {}
                        }
                    }
                    self.env.pop_scope();
                    if should_break { break; }
                }
                Ok(Signal::None)
            }

            Statement::For { var, iterable, body } => {
                let iter_val = self.eval_expression(iterable)?;
                match iter_val {
                    Value::Array(items) => {
                        for item in items {
                            self.env.push_scope();
                            self.env.define(var, item);
                            let mut should_break = false;
                            for s in body {
                                match self.exec_statement(s)? {
                                    Signal::Break => { should_break = true; break; }
                                    Signal::Continue => break,
                                    Signal::Return(v) => { self.env.pop_scope(); return Ok(Signal::Return(v)); }
                                    Signal::None => {}
                                }
                            }
                            self.env.pop_scope();
                            if should_break { break; }
                        }
                    }
                    _ => return Err("For loop requires an iterable (array or range)".to_string()),
                }
                Ok(Signal::None)
            }

            Statement::Break => Ok(Signal::Break),
            Statement::Continue => Ok(Signal::Continue),

            Statement::Struct { name, .. } => {
                // For now, just register the struct name
                self.env.define(name, Value::None);
                Ok(Signal::None)
            }

            Statement::Enum { name, variants } => {
                let variant_names = variants.iter().map(|v| v.name.clone()).collect();
                let enum_type = Value::EnumType {
                    name: name.clone(),
                    variants: variant_names,
                };
                self.env.define(name, enum_type);
                Ok(Signal::None)
            }
        }
    }

    /// Evaluate an expression and return its value
    fn eval_expression(&mut self, expr: &Expression) -> Result<Value, String> {
        match expr {
            Expression::IntLiteral(n) => Ok(Value::Int(*n)),
            Expression::FloatLiteral(f) => Ok(Value::Float(*f)),
            Expression::StringLiteral(s) => {
                // Handle string interpolation: replace {var} with values
                let interpolated = self.interpolate_string(s)?;
                Ok(Value::Str(interpolated))
            }
            Expression::BoolLiteral(b) => Ok(Value::Bool(*b)),

            Expression::Identifier(name) => {
                self.env.get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable '{}'", name))
            }

            Expression::BinaryOp { left, op, right } => {
                let l = self.eval_expression(left)?;
                let r = self.eval_expression(right)?;
                self.eval_binary_op(&l, op, &r)
            }

            Expression::UnaryOp { op, operand } => {
                let val = self.eval_expression(operand)?;
                match op {
                    UnaryOperator::Neg => match val {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err("Cannot negate non-numeric value".to_string()),
                    },
                    UnaryOperator::Not => match val {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        _ => Err("Cannot apply ! to non-boolean".to_string()),
                    },
                }
            }

            Expression::Call { function, args } => {
                // Built-in functions
                if let Expression::Identifier(name) = function.as_ref() {
                    match name.as_str() {
                        "println" => {
                            let mut parts = Vec::new();
                            for arg in args {
                                let val = self.eval_expression(arg)?;
                                parts.push(format!("{}", val));
                            }
                            let output = parts.join(" ");
                            println!("{}", output);
                            self.output.push(output);
                            return Ok(Value::None);
                        }
                        "print" => {
                            let mut parts = Vec::new();
                            for arg in args {
                                let val = self.eval_expression(arg)?;
                                parts.push(format!("{}", val));
                            }
                            let output = parts.join(" ");
                            print!("{}", output);
                            self.output.push(output);
                            return Ok(Value::None);
                        }
                        "len" => {
                            if args.len() != 1 {
                                return Err("len() takes exactly 1 argument".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            return match val {
                                Value::Str(s) => Ok(Value::Int(s.len() as i64)),
                                Value::Array(a) => Ok(Value::Int(a.len() as i64)),
                                _ => Err("len() requires a string or array".to_string()),
                            }
                        }
                        "push" => {
                            if args.len() != 2 {
                                return Err("push() takes 2 arguments (array, value)".to_string());
                            }
                            let mut arr = self.eval_expression(&args[0])?;
                            let val = self.eval_expression(&args[1])?;
                            return match &mut arr {
                                Value::Array(a) => {
                                    a.push(val);
                                    Ok(arr.clone())
                                }
                                _ => Err("push() requires an array as first argument".to_string()),
                            }
                        }
                        "type_of" => {
                            if args.len() != 1 {
                                return Err("type_of() takes exactly 1 argument".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            let type_name = match val {
                                Value::Int(_) => "i64",
                                Value::Float(_) => "f64",
                                Value::Str(_) => "str",
                                Value::Bool(_) => "bool",
                                Value::Array(_) => "array",
                                Value::None => "None",
                                Value::Function { .. } => "fn",
                                Value::Struct { .. } => "struct",
                                Value::EnumType { .. } => "enum",
                                Value::Variant { .. } => "variant",
                            };
                            return Ok(Value::Str(type_name.to_string()));
                        }
                        "to_string" => {
                            if args.len() != 1 {
                                return Err("to_string() takes exactly 1 argument".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            return Ok(Value::Str(format!("{}", val)));
                        }
                        _ => {}
                    }
                }

                // User-defined functions
                let func_val = self.eval_expression(function)?;
                match func_val {
                    Value::Function { params, body, .. } => {
                        if args.len() != params.len() {
                            return Err(format!(
                                "Expected {} arguments, got {}",
                                params.len(), args.len()
                            ));
                        }

                        // Evaluate arguments
                        let mut arg_values = Vec::new();
                        for arg in args {
                            arg_values.push(self.eval_expression(arg)?);
                        }

                        // Create new scope with parameters
                        self.env.push_scope();
                        for (param, val) in params.iter().zip(arg_values) {
                            self.env.define(&param.name, val);
                        }

                        // Execute body
                        let mut result = Value::None;
                        for stmt in &body {
                            match self.exec_statement(stmt)? {
                                Signal::Return(v) => { result = v; break; }
                                _ => {}
                            }
                        }

                        self.env.pop_scope();
                        Ok(result)
                    }
                    _ => Err(format!("'{}' is not a function", func_val)),
                }
            }

            Expression::Assign { target, value } => {
                let val = self.eval_expression(value)?;
                if let Expression::Identifier(name) = target.as_ref() {
                    self.env.set(name, val.clone())?;
                    Ok(val)
                } else {
                    Err("Invalid assignment target".to_string())
                }
            }

            Expression::Array(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.eval_expression(elem)?);
                }
                Ok(Value::Array(values))
            }

            Expression::Range { start, end, inclusive } => {
                let s = self.eval_expression(start)?;
                let e = self.eval_expression(end)?;
                match (s, e) {
                    (Value::Int(s), Value::Int(e)) => {
                        let range: Vec<Value> = if *inclusive {
                            (s..=e).map(Value::Int).collect()
                        } else {
                            (s..e).map(Value::Int).collect()
                        };
                        Ok(Value::Array(range))
                    }
                    _ => Err("Range requires integer values".to_string()),
                }
            }

            Expression::Pipe { left, right } => {
                let left_val = self.eval_expression(left)?;
                // Pipe operator: left_val becomes the first argument to right
                match right.as_ref() {
                    Expression::Call { function, args } => {
                        let mut new_args = vec![Expression::IntLiteral(0)]; // placeholder
                        new_args.extend(args.clone());
                        // We need a special approach - evaluate left, then call right with it
                        let func_val = self.eval_expression(function)?;
                        match func_val {
                            Value::Function { params, body, .. } => {
                                self.env.push_scope();
                                self.env.define(&params[0].name, left_val);
                                for (i, arg) in args.iter().enumerate() {
                                    if i + 1 < params.len() {
                                        let val = self.eval_expression(arg)?;
                                        self.env.define(&params[i + 1].name, val);
                                    }
                                }
                                let mut result = Value::None;
                                for stmt in &body {
                                    match self.exec_statement(stmt)? {
                                        Signal::Return(v) => { result = v; break; }
                                        _ => {}
                                    }
                                }
                                self.env.pop_scope();
                                Ok(result)
                            }
                            _ => Err("Pipe target must be a function".to_string()),
                        }
                    }
                    Expression::Identifier(name) => {
                        let func_val = self.env.get(name).cloned()
                            .ok_or_else(|| format!("Undefined function '{}'", name))?;
                        match func_val {
                            Value::Function { params, body, .. } => {
                                self.env.push_scope();
                                if !params.is_empty() {
                                    self.env.define(&params[0].name, left_val);
                                }
                                let mut result = Value::None;
                                for stmt in &body {
                                    match self.exec_statement(stmt)? {
                                        Signal::Return(v) => { result = v; break; }
                                        _ => {}
                                    }
                                }
                                self.env.pop_scope();
                                Ok(result)
                            }
                            _ => Err(format!("'{}' is not a function", name)),
                        }
                    }
                    _ => Err("Pipe operator requires a function on the right".to_string()),
                }
            }

            Expression::MemberAccess { object, member } => {
                let val = self.eval_expression(object)?;
                match val {
                    Value::EnumType { name, variants } => {
                        if variants.contains(member) {
                            Ok(Value::Variant {
                                enum_name: name,
                                variant: member.clone(),
                                values: Vec::new(),
                            })
                        } else {
                            Err(format!("Enum '{}' has no variant '{}'", name, member))
                        }
                    }
                    Value::Struct { fields, .. } => {
                        fields.get(member)
                            .cloned()
                            .ok_or_else(|| format!("Struct has no field '{}'", member))
                    }
                    Value::Str(s) if member == "len" => Ok(Value::Int(s.len() as i64)),
                    Value::Array(a) if member == "len" => Ok(Value::Int(a.len() as i64)),
                    _ => Err(format!("Cannot access member '{}' on {}", member, val)),
                }
            }

            Expression::StructInit { name, fields } => {
                let mut field_values = HashMap::new();
                for (field_name, expr) in fields {
                    let val = self.eval_expression(expr)?;
                    field_values.insert(field_name.clone(), val);
                }
                Ok(Value::Struct {
                    name: name.clone(),
                    fields: field_values,
                })
            }

            Expression::Index { object, index } => {
                let obj = self.eval_expression(object)?;
                let idx = self.eval_expression(index)?;
                match (obj, idx) {
                    (Value::Array(arr), Value::Int(i)) => {
                        let i = i as usize;
                        arr.get(i).cloned()
                            .ok_or_else(|| format!("Index {} out of bounds (len: {})", i, arr.len()))
                    }
                    (Value::Str(s), Value::Int(i)) => {
                        let i = i as usize;
                        s.chars().nth(i)
                            .map(|c| Value::Str(c.to_string()))
                            .ok_or_else(|| format!("Index {} out of bounds", i))
                    }
                    _ => Err("Cannot index non-array/string value".to_string()),
                }
            }

            Expression::Match { value, arms } => {
                let val = self.eval_expression(value)?;
                for arm in arms {
                    let mut bindings = HashMap::new();
                    if self.pattern_matches(&val, &arm.pattern, &mut bindings)? {
                        self.env.push_scope();
                        for (name, value) in bindings {
                            self.env.define(&name, value);
                        }
                        let result = self.eval_expression(&arm.body);
                        self.env.pop_scope();
                        return result;
                    }
                }
                Err("Non-exhaustive match: no arm matched".to_string())
            }

            Expression::Block(stmts) => {
                self.env.push_scope();
                let mut result = Value::None;
                for stmt in stmts {
                    match self.exec_statement(stmt)? {
                        Signal::Return(v) => { result = v; break; }
                        _ => {}
                    }
                }
                self.env.pop_scope();
                Ok(result)
            }

            Expression::Lambda { params, body } => {
                let func_params: Vec<Param> = params.iter().map(|p| Param {
                    name: p.clone(),
                    type_name: "any".to_string(),
                }).collect();
                Ok(Value::Function {
                    name: "<lambda>".to_string(),
                    params: func_params,
                    body: vec![Statement::Return(Some(*body.clone()))],
                })
            }
        }
    }

    /// String interpolation: replace {expr} with values
    fn interpolate_string(&self, s: &str) -> Result<String, String> {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                // Collect variable name
                let mut var_name = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next();
                        break;
                    }
                    var_name.push(next);
                    chars.next();
                }
                // Look up variable
                if let Some(val) = self.env.get(&var_name) {
                    result.push_str(&format!("{}", val));
                } else {
                    result.push('{');
                    result.push_str(&var_name);
                    result.push('}');
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    fn eval_binary_op(&self, left: &Value, op: &BinaryOperator, right: &Value) -> Result<Value, String> {
        match (left, right) {
            // Integer arithmetic
            (Value::Int(a), Value::Int(b)) => match op {
                BinaryOperator::Add => Ok(Value::Int(a + b)),
                BinaryOperator::Sub => Ok(Value::Int(a - b)),
                BinaryOperator::Mul => Ok(Value::Int(a * b)),
                BinaryOperator::Div => {
                    if *b == 0 { return Err("Division by zero".to_string()); }
                    Ok(Value::Int(a / b))
                }
                BinaryOperator::Mod => Ok(Value::Int(a % b)),
                BinaryOperator::Eq => Ok(Value::Bool(a == b)),
                BinaryOperator::NotEq => Ok(Value::Bool(a != b)),
                BinaryOperator::Less => Ok(Value::Bool(a < b)),
                BinaryOperator::Greater => Ok(Value::Bool(a > b)),
                BinaryOperator::LessEq => Ok(Value::Bool(a <= b)),
                BinaryOperator::GreaterEq => Ok(Value::Bool(a >= b)),
                _ => Err(format!("Cannot apply {:?} to integers", op)),
            },
            // Float arithmetic
            (Value::Float(a), Value::Float(b)) => match op {
                BinaryOperator::Add => Ok(Value::Float(a + b)),
                BinaryOperator::Sub => Ok(Value::Float(a - b)),
                BinaryOperator::Mul => Ok(Value::Float(a * b)),
                BinaryOperator::Div => Ok(Value::Float(a / b)),
                BinaryOperator::Eq => Ok(Value::Bool(a == b)),
                BinaryOperator::NotEq => Ok(Value::Bool(a != b)),
                BinaryOperator::Less => Ok(Value::Bool(a < b)),
                BinaryOperator::Greater => Ok(Value::Bool(a > b)),
                _ => Err(format!("Cannot apply {:?} to floats", op)),
            },
            // Mixed int/float
            (Value::Int(a), Value::Float(b)) => self.eval_binary_op(&Value::Float(*a as f64), op, &Value::Float(*b)),
            (Value::Float(a), Value::Int(b)) => self.eval_binary_op(&Value::Float(*a), op, &Value::Float(*b as f64)),
            // String concatenation
            (Value::Str(a), Value::Str(b)) => match op {
                BinaryOperator::Add => Ok(Value::Str(format!("{}{}", a, b))),
                BinaryOperator::Eq => Ok(Value::Bool(a == b)),
                BinaryOperator::NotEq => Ok(Value::Bool(a != b)),
                _ => Err(format!("Cannot apply {:?} to strings", op)),
            },
            // Boolean operations
            (Value::Bool(a), Value::Bool(b)) => match op {
                BinaryOperator::And => Ok(Value::Bool(*a && *b)),
                BinaryOperator::Or => Ok(Value::Bool(*a || *b)),
                BinaryOperator::Eq => Ok(Value::Bool(a == b)),
                BinaryOperator::NotEq => Ok(Value::Bool(a != b)),
                _ => Err(format!("Cannot apply {:?} to booleans", op)),
            },
            // Enum Variant equality
            (Value::Variant { enum_name: e1, variant: v1, values: vals1 }, 
             Value::Variant { enum_name: e2, variant: v2, values: vals2 }) => {
                match op {
                    BinaryOperator::Eq => {
                        if e1 != e2 || v1 != v2 || vals1.len() != vals2.len() {
                            Ok(Value::Bool(false))
                        } else {
                            // Recursively check values
                            let mut equal = true;
                            for (val1, val2) in vals1.iter().zip(vals2.iter()) {
                                let res = self.eval_binary_op(val1, &BinaryOperator::Eq, val2)?;
                                if let Value::Bool(b) = res {
                                    if !b { equal = false; break; }
                                } else {
                                    equal = false; break;
                                }
                            }
                            Ok(Value::Bool(equal))
                        }
                    }
                    BinaryOperator::NotEq => {
                        let eq_res = self.eval_binary_op(left, &BinaryOperator::Eq, right)?;
                        if let Value::Bool(b) = eq_res {
                            Ok(Value::Bool(!b))
                        } else {
                            Ok(Value::Bool(true))
                        }
                    }
                    _ => Err(format!("Cannot apply {:?} to enum variants", op)),
                }
            },
            _ => Err(format!("Type mismatch: cannot apply {:?} to {:?} and {:?}", op, left, right)),
        }
    }

    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Str(s) => !s.is_empty(),
            Value::None => false,
            _ => true,
        }
    }

    fn pattern_matches(&self, value: &Value, pattern: &Pattern, bindings: &mut HashMap<String, Value>) -> Result<bool, String> {
        match pattern {
            Pattern::Wildcard => Ok(true),
            Pattern::Identifier(name) => {
                // Bind value to name
                bindings.insert(name.clone(), value.clone());
                Ok(true)
            }
            Pattern::Literal(expr) => {
                match (value, expr) {
                    (Value::Int(a), Expression::IntLiteral(b)) => Ok(a == b),
                    (Value::Str(a), Expression::StringLiteral(b)) => Ok(a == b),
                    (Value::Bool(a), Expression::BoolLiteral(b)) => Ok(a == b),
                    _ => Ok(false),
                }
            }
            Pattern::EnumVariant { name: pat_name, fields: pat_fields } => {
                if let Value::Variant { enum_name, variant, values } = value {
                    let parts: Vec<&str> = pat_name.split("::").collect();
                    let (expected_enum, expected_variant) = if parts.len() == 2 {
                        (Some(parts[0]), parts[1])
                    } else {
                        (None, parts[0])
                    };

                    if let Some(e) = expected_enum {
                        if e != enum_name { return Ok(false); }
                    }
                    if expected_variant != variant { return Ok(false); }
                    if pat_fields.len() != values.len() { return Ok(false); }

                    for (pat, val) in pat_fields.iter().zip(values.iter()) {
                        if !self.pattern_matches(val, pat, bindings)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn run_vryn(source: &str) -> (Result<(), String>, Vec<String>) {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut interpreter = Interpreter::new();
        let result = interpreter.run(&program);
        let output = interpreter.get_output().clone();
        (result, output)
    }

    #[test]
    fn test_hello_world() {
        let (result, output) = run_vryn(r#"println("Hello, World!")"#);
        assert!(result.is_ok());
        assert_eq!(output[0], "Hello, World!");
    }

    #[test]
    fn test_variables_and_math() {
        let (result, output) = run_vryn(r#"
            let x = 10
            let y = 20
            println(x + y)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "30");
    }

    #[test]
    fn test_function_call() {
        let (result, output) = run_vryn(r#"
            fn add(a: i32, b: i32) -> i32 {
                return a + b
            }
            let result = add(5, 3)
            println(result)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "8");
    }

    #[test]
    fn test_if_else() {
        let (result, output) = run_vryn(r#"
            let x = 10
            if x > 5 {
                println("big")
            } else {
                println("small")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "big");
    }

    #[test]
    fn test_while_loop() {
        let (result, output) = run_vryn(r#"
            let mut i = 0
            while i < 5 {
                i = i + 1
            }
            println(i)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "5");
    }

    #[test]
    fn test_for_loop() {
        let (result, output) = run_vryn(r#"
            for i in 0..5 {
                println(i)
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output.len(), 5);
        assert_eq!(output[0], "0");
        assert_eq!(output[4], "4");
    }

    #[test]
    fn test_string_interpolation() {
        let (result, output) = run_vryn(r#"
            let name = "Vryn"
            println("Hello, {name}!")
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "Hello, Vryn!");
    }

    #[test]
    fn test_array() {
        let (result, output) = run_vryn(r#"
            let arr = [1, 2, 3, 4, 5]
            println(arr[2])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
    }

    #[test]
    fn test_fibonacci() {
        let (result, output) = run_vryn(r#"
            fn fib(n: i32) -> i32 {
                if n <= 1 {
                    return n
                }
                return fib(n - 1) + fib(n - 2)
            }
            println(fib(10))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "55");
    }

    #[test]
    fn test_nested_functions() {
        let (result, output) = run_vryn(r#"
            fn double(x: i32) -> i32 {
                return x * 2
            }
            fn add_one(x: i32) -> i32 {
                return x + 1
            }
            println(double(add_one(4)))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "10");
    }
}
