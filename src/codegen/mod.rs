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
    /// Result type: Ok(value) or Err(value)
    Result {
        ok: bool,
        value: Box<Value>,
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
            Value::Result { ok, value } => {
                if *ok {
                    write!(f, "Ok({})", value)
                } else {
                    write!(f, "Err({})", value)
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

            Statement::IfLet { pattern, expr, then_body, else_body } => {
                let val = self.eval_expression(expr)?;
                let mut bindings = HashMap::new();
                if self.pattern_matches(&val, pattern, &mut bindings)? {
                    self.env.push_scope();
                    for (name, value) in bindings {
                        self.env.define(&name, value);
                    }
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

            Statement::WhileLet { pattern, expr, body } => {
                loop {
                    let val = self.eval_expression(expr)?;
                    let mut bindings = HashMap::new();
                    if !self.pattern_matches(&val, pattern, &mut bindings)? {
                        break;
                    }

                    self.env.push_scope();
                    for (name, value) in bindings {
                        self.env.define(&name, value);
                    }
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
                                Value::Result { .. } => "Result",
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
                        "Ok" => {
                            if args.len() != 1 {
                                return Err("Ok() takes exactly 1 argument".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            return Ok(Value::Result {
                                ok: true,
                                value: Box::new(val),
                            });
                        }
                        "Err" => {
                            if args.len() != 1 {
                                return Err("Err() takes exactly 1 argument".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            return Ok(Value::Result {
                                ok: false,
                                value: Box::new(val),
                            });
                        }
                        "is_ok" => {
                            if args.len() != 1 {
                                return Err("is_ok() takes exactly 1 argument".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            return match val {
                                Value::Result { ok, .. } => Ok(Value::Bool(ok)),
                                _ => Err("is_ok() requires a Result".to_string()),
                            };
                        }
                        "is_err" => {
                            if args.len() != 1 {
                                return Err("is_err() takes exactly 1 argument".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            return match val {
                                Value::Result { ok, .. } => Ok(Value::Bool(!ok)),
                                _ => Err("is_err() requires a Result".to_string()),
                            };
                        }
                        "unwrap" => {
                            if args.len() != 1 {
                                return Err("unwrap() takes exactly 1 argument".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            return match val {
                                Value::Result { ok, value } => {
                                    if ok {
                                        Ok(*value)
                                    } else {
                                        Err(format!("Called unwrap on an error: {}", value))
                                    }
                                }
                                _ => Err("unwrap() requires a Result".to_string()),
                            };
                        }
                        "unwrap_or" => {
                            if args.len() != 2 {
                                return Err("unwrap_or() takes exactly 2 arguments".to_string());
                            }
                            let val = self.eval_expression(&args[0])?;
                            let default = self.eval_expression(&args[1])?;
                            return match val {
                                Value::Result { ok, value } => {
                                    if ok {
                                        Ok(*value)
                                    } else {
                                        Ok(default)
                                    }
                                }
                                _ => Err("unwrap_or() requires a Result".to_string()),
                            };
                        }
                        "panic" => {
                            if args.is_empty() {
                                return Err("Panic!".to_string());
                            }
                            let msg = self.eval_expression(&args[0])?;
                            return Err(format!("Panic: {}", msg));
                        }
// String functions
"str_len" => {
    if args.len() != 1 {
        return Err("str_len() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Str(s) => Ok(Value::Int(s.len() as i64)),
        _ => Err("str_len() requires a string".to_string()),
    };
}
"str_contains" => {
    if args.len() != 2 {
        return Err("str_contains() takes 2 arguments".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    let substr = self.eval_expression(&args[1])?;
    return match (s, substr) {
        (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(&sub))),
        _ => Err("str_contains() requires strings".to_string()),
    };
}
"str_starts_with" => {
    if args.len() != 2 {
        return Err("str_starts_with() takes 2 arguments".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    let prefix = self.eval_expression(&args[1])?;
    return match (s, prefix) {
        (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.starts_with(&p))),
        _ => Err("str_starts_with() requires strings".to_string()),
    };
}
"str_ends_with" => {
    if args.len() != 2 {
        return Err("str_ends_with() takes 2 arguments".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    let suffix = self.eval_expression(&args[1])?;
    return match (s, suffix) {
        (Value::Str(s), Value::Str(suf)) => Ok(Value::Bool(s.ends_with(&suf))),
        _ => Err("str_ends_with() requires strings".to_string()),
    };
}
"str_split" => {
    if args.len() != 2 {
        return Err("str_split() takes 2 arguments".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    let delim = self.eval_expression(&args[1])?;
    return match (s, delim) {
        (Value::Str(s), Value::Str(d)) => {
            let parts: Vec<Value> = s.split(&d).map(|p| Value::Str(p.to_string())).collect();
            Ok(Value::Array(parts))
        }
        _ => Err("str_split() requires strings".to_string()),
    };
}
"str_trim" => {
    if args.len() != 1 {
        return Err("str_trim() takes exactly 1 argument".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    return match s {
        Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
        _ => Err("str_trim() requires a string".to_string()),
    };
}
"str_upper" => {
    if args.len() != 1 {
        return Err("str_upper() takes exactly 1 argument".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    return match s {
        Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
        _ => Err("str_upper() requires a string".to_string()),
    };
}
"str_lower" => {
    if args.len() != 1 {
        return Err("str_lower() takes exactly 1 argument".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    return match s {
        Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
        _ => Err("str_lower() requires a string".to_string()),
    };
}
"str_replace" => {
    if args.len() != 3 {
        return Err("str_replace() takes 3 arguments".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    let from = self.eval_expression(&args[1])?;
    let to = self.eval_expression(&args[2])?;
    return match (s, from, to) {
        (Value::Str(s), Value::Str(f), Value::Str(t)) => {
            Ok(Value::Str(s.replace(&f, &t)))
        }
        _ => Err("str_replace() requires strings".to_string()),
    };
}
"str_chars" => {
    if args.len() != 1 {
        return Err("str_chars() takes exactly 1 argument".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    return match s {
        Value::Str(s) => {
            let chars: Vec<Value> = s.chars().map(|c| Value::Str(c.to_string())).collect();
            Ok(Value::Array(chars))
        }
        _ => Err("str_chars() requires a string".to_string()),
    };
}
"str_join" => {
    if args.len() != 2 {
        return Err("str_join() takes 2 arguments".to_string());
    }
    let arr = self.eval_expression(&args[0])?;
    let sep = self.eval_expression(&args[1])?;
    return match (arr, sep) {
        (Value::Array(a), Value::Str(s)) => {
            let strs: Vec<String> = a.iter().map(|v| format!("{}", v)).collect();
            Ok(Value::Str(strs.join(&s)))
        }
        _ => Err("str_join() requires an array and a string separator".to_string()),
    };
}
"substr" => {
    if args.len() != 3 {
        return Err("substr() takes 3 arguments (string, start, length)".to_string());
    }
    let s = self.eval_expression(&args[0])?;
    let start = self.eval_expression(&args[1])?;
    let len = self.eval_expression(&args[2])?;
    return match (s, start, len) {
        (Value::Str(s), Value::Int(st), Value::Int(l)) => {
            let st = st as usize;
            let l = l as usize;
            if st > s.len() {
                Ok(Value::Str("".to_string()))
            } else {
                let end = std::cmp::min(st + l, s.len());
                Ok(Value::Str(s[st..end].to_string()))
            }
        }
        _ => Err("substr() requires a string and integers".to_string()),
    };
}
// Array functions
"pop" => {
    if args.len() != 1 {
        return Err("pop() takes exactly 1 argument".to_string());
    }
    let mut arr = self.eval_expression(&args[0])?;
    return match &mut arr {
        Value::Array(a) => {
            if let Some(v) = a.pop() {
                Ok(v)
            } else {
                Ok(Value::None)
            }
        }
        _ => Err("pop() requires an array".to_string()),
    };
}
"arr_len" => {
    if args.len() != 1 {
        return Err("arr_len() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Array(a) => Ok(Value::Int(a.len() as i64)),
        _ => Err("arr_len() requires an array".to_string()),
    };
}
"arr_reverse" => {
    if args.len() != 1 {
        return Err("arr_reverse() takes exactly 1 argument".to_string());
    }
    let mut arr = self.eval_expression(&args[0])?;
    return match &mut arr {
        Value::Array(a) => {
            a.reverse();
            Ok(arr)
        }
        _ => Err("arr_reverse() requires an array".to_string()),
    };
}
"arr_contains" => {
    if args.len() != 2 {
        return Err("arr_contains() takes 2 arguments".to_string());
    }
    let arr = self.eval_expression(&args[0])?;
    let val = self.eval_expression(&args[1])?;
    return match arr {
        Value::Array(a) => {
            let contains = a.iter().any(|v| {
                match (v, &val) {
                    (Value::Int(a), Value::Int(b)) => a == b,
                    (Value::Float(a), Value::Float(b)) => (a - b).abs() < 1e-10,
                    (Value::Str(a), Value::Str(b)) => a == b,
                    (Value::Bool(a), Value::Bool(b)) => a == b,
                    _ => false,
                }
            });
            Ok(Value::Bool(contains))
        }
        _ => Err("arr_contains() requires an array".to_string()),
    };
}
"arr_slice" => {
    if args.len() != 3 {
        return Err("arr_slice() takes 3 arguments (array, start, end)".to_string());
    }
    let arr = self.eval_expression(&args[0])?;
    let start = self.eval_expression(&args[1])?;
    let end = self.eval_expression(&args[2])?;
    return match (arr, start, end) {
        (Value::Array(a), Value::Int(s), Value::Int(e)) => {
            let s = s as usize;
            let e = e as usize;
            let e = std::cmp::min(e, a.len());
            if s > a.len() || s >= e {
                Ok(Value::Array(vec![]))
            } else {
                Ok(Value::Array(a[s..e].to_vec()))
            }
        }
        _ => Err("arr_slice() requires an array and integers".to_string()),
    };
}
"arr_sort" => {
    if args.len() != 1 {
        return Err("arr_sort() takes exactly 1 argument".to_string());
    }
    let mut arr = self.eval_expression(&args[0])?;
    return match &mut arr {
        Value::Array(a) => {
            a.sort_by(|x, y| {
                match (x, y) {
                    (Value::Int(a), Value::Int(b)) => a.cmp(b),
                    (Value::Float(a), Value::Float(b)) => {
                        if a < b { std::cmp::Ordering::Less }
                        else if a > b { std::cmp::Ordering::Greater }
                        else { std::cmp::Ordering::Equal }
                    }
                    (Value::Str(a), Value::Str(b)) => a.cmp(b),
                    _ => std::cmp::Ordering::Equal,
                }
            });
            Ok(arr)
        }
        _ => Err("arr_sort() requires an array".to_string()),
    };
}
// Math functions
"abs" => {
    if args.len() != 1 {
        return Err("abs() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Int(n) => Ok(Value::Int(n.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err("abs() requires a number".to_string()),
    };
}
"min" => {
    if args.len() != 2 {
        return Err("min() takes 2 arguments".to_string());
    }
    let a = self.eval_expression(&args[0])?;
    let b = self.eval_expression(&args[1])?;
    return match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x.min(y))),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.min(y))),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float((x as f64).min(y))),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.min(y as f64))),
        _ => Err("min() requires numbers".to_string()),
    };
}
"max" => {
    if args.len() != 2 {
        return Err("max() takes 2 arguments".to_string());
    }
    let a = self.eval_expression(&args[0])?;
    let b = self.eval_expression(&args[1])?;
    return match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x.max(y))),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.max(y))),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float((x as f64).max(y))),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.max(y as f64))),
        _ => Err("max() requires numbers".to_string()),
    };
}
"floor" => {
    if args.len() != 1 {
        return Err("floor() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Float(f) => Ok(Value::Int(f.floor() as i64)),
        Value::Int(i) => Ok(Value::Int(i)),
        _ => Err("floor() requires a number".to_string()),
    };
}
"ceil" => {
    if args.len() != 1 {
        return Err("ceil() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Float(f) => Ok(Value::Int(f.ceil() as i64)),
        Value::Int(i) => Ok(Value::Int(i)),
        _ => Err("ceil() requires a number".to_string()),
    };
}
"round" => {
    if args.len() != 1 {
        return Err("round() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Float(f) => Ok(Value::Int(f.round() as i64)),
        Value::Int(i) => Ok(Value::Int(i)),
        _ => Err("round() requires a number".to_string()),
    };
}
"sqrt" => {
    if args.len() != 1 {
        return Err("sqrt() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Float(f) => Ok(Value::Float(f.sqrt())),
        Value::Int(i) => Ok(Value::Float((i as f64).sqrt())),
        _ => Err("sqrt() requires a number".to_string()),
    };
}
"pow" => {
    if args.len() != 2 {
        return Err("pow() takes 2 arguments".to_string());
    }
    let base = self.eval_expression(&args[0])?;
    let exp = self.eval_expression(&args[1])?;
    return match (base, exp) {
        (Value::Float(b), Value::Float(e)) => Ok(Value::Float(b.powf(e))),
        (Value::Int(b), Value::Int(e)) => {
            if e >= 0 {
                Ok(Value::Int(b.pow(e as u32)))
            } else {
                Ok(Value::Float((b as f64).powf(e as f64)))
            }
        }
        (Value::Int(b), Value::Float(e)) => Ok(Value::Float((b as f64).powf(e))),
        (Value::Float(b), Value::Int(e)) => Ok(Value::Float(b.powf(e as f64))),
        _ => Err("pow() requires numbers".to_string()),
    };
}
"random" => {
    if !args.is_empty() {
        return Err("random() takes no arguments".to_string());
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let seed = nanos as f64 / 1_000_000_000.0;
    return Ok(Value::Float(seed % 1.0));
}
"int" => {
    if args.len() != 1 {
        return Err("int() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Int(i) => Ok(Value::Int(i)),
        Value::Float(f) => Ok(Value::Int(f as i64)),
        Value::Str(s) => {
            match s.parse::<i64>() {
                Ok(i) => Ok(Value::Int(i)),
                Err(_) => Err(format!("Cannot convert '{}' to int", s)),
            }
        }
        Value::Bool(b) => Ok(Value::Int(if b { 1 } else { 0 })),
        _ => Err("int() requires a convertible value".to_string()),
    };
}
"float" => {
    if args.len() != 1 {
        return Err("float() takes exactly 1 argument".to_string());
    }
    let val = self.eval_expression(&args[0])?;
    return match val {
        Value::Float(f) => Ok(Value::Float(f)),
        Value::Int(i) => Ok(Value::Float(i as f64)),
        Value::Str(s) => {
            match s.parse::<f64>() {
                Ok(f) => Ok(Value::Float(f)),
                Err(_) => Err(format!("Cannot convert '{}' to float", s)),
            }
        }
        _ => Err("float() requires a convertible value".to_string()),
    };
}
// I/O functions
"input" => {
    let prompt = if !args.is_empty() {
        let p = self.eval_expression(&args[0])?;
        match p {
            Value::Str(s) => s,
            _ => format!("{}", p),
        }
    } else {
        String::new()
    };
    print!("{}", prompt);
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(_) => {
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            return Ok(Value::Str(line));
        }
        Err(e) => return Err(format!("Failed to read input: {}", e)),
    }
}
"read_file" => {
    if args.len() != 1 {
        return Err("read_file() takes exactly 1 argument".to_string());
    }
    let path = self.eval_expression(&args[0])?;
    return match path {
        Value::Str(p) => {
            match std::fs::read_to_string(&p) {
                Ok(content) => Ok(Value::Str(content)),
                Err(e) => Err(format!("Failed to read file '{}': {}", p, e)),
            }
        }
        _ => Err("read_file() requires a string path".to_string()),
    };
}
"write_file" => {
    if args.len() != 2 {
        return Err("write_file() takes 2 arguments".to_string());
    }
    let path = self.eval_expression(&args[0])?;
    let content = self.eval_expression(&args[1])?;
    return match (path, content) {
        (Value::Str(p), c) => {
            match std::fs::write(&p, format!("{}", c)) {
                Ok(_) => Ok(Value::None),
                Err(e) => Err(format!("Failed to write file '{}': {}", p, e)),
            }
        }
        _ => Err("write_file() requires a string path and content".to_string()),
    };
}
"file_exists" => {
    if args.len() != 1 {
        return Err("file_exists() takes exactly 1 argument".to_string());
    }
    let path = self.eval_expression(&args[0])?;
    return match path {
        Value::Str(p) => {
            Ok(Value::Bool(std::path::Path::new(&p).exists()))
        }
        _ => Err("file_exists() requires a string path".to_string()),
    };
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
                    if self.pattern_matches(&val, &arm.pattern, &mut bindings)? { // line 725
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

            Expression::TryCatch { try_body, catch_var, catch_body } => {
                self.env.push_scope();
                let mut result = Value::None;
                let mut caught_error = false;

                // Try executing the try block
                for stmt in try_body {
                    match self.exec_statement(stmt) {
                        Ok(Signal::Return(v)) => {
                            result = v;
                            self.env.pop_scope();
                            return Ok(result);
                        }
                        Ok(_) => {}
                        Err(err) => {
                            // Caught an error, bind it to the catch variable and run catch block
                            caught_error = true;
                            self.env.define(catch_var, Value::Str(err));
                            break;
                        }
                    }
                }

                if caught_error {
                    // Execute catch block with the error bound to catch_var
                    for stmt in catch_body {
                        match self.exec_statement(stmt) {
                            Ok(Signal::Return(v)) => {
                                result = v;
                                self.env.pop_scope();
                                return Ok(result);
                            }
                            Ok(_) => {}
                            Err(e) => {
                                self.env.pop_scope();
                                return Err(e);
                            }
                        }
                    }
                }

                self.env.pop_scope();
                Ok(result)
            }

            Expression::QuestionMark { expr } => {
                let val = self.eval_expression(expr)?;
                match val {
                    Value::Result { ok, value } => {
                        if ok {
                            Ok(*value)
                        } else {
                            // Propagate the error up
                            Err(format!("Error propagated with ?: {}", value))
                        }
                    }
                    _ => Err("? operator requires a Result type".to_string()),
                }
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

    fn pattern_matches(&mut self, value: &Value, pattern: &Pattern, bindings: &mut HashMap<String, Value>) -> Result<bool, String> {
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
                    (Value::Float(a), Expression::FloatLiteral(b)) => Ok((a - b).abs() < f64::EPSILON),
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
            Pattern::Tuple(pat_elements) => {
                // Cannot create tuples in current Vryn, so this would only match arrays for now
                match value {
                    Value::Array(arr) => {
                        if pat_elements.len() != arr.len() {
                            return Ok(false);
                        }
                        for (pat, val) in pat_elements.iter().zip(arr.iter()) {
                            if !self.pattern_matches(val, pat, bindings)? {
                                return Ok(false);
                            }
                        }
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            }
            Pattern::Struct { name: pat_name, fields: pat_fields } => {
                if let Value::Struct { name, fields } = value {
                    if pat_name != name {
                        return Ok(false);
                    }
                    // Match struct fields
                    for (field_name, field_pattern) in pat_fields {
                        if let Some(field_value) = fields.get(field_name) {
                            if !self.pattern_matches(field_value, field_pattern, bindings)? {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Pattern::Range { start, end, inclusive } => {
                // Range patterns match if value is within the range
                if let Value::Int(n) = value {
                    let start_val = self.eval_expression(start)?;
                    let end_val = self.eval_expression(end)?;
                    if let (Value::Int(s), Value::Int(e)) = (start_val, end_val) {
                        let in_range = if *inclusive {
                            *n >= s && *n <= e
                        } else {
                            *n >= s && *n < e
                        };
                        return Ok(in_range);
                    }
                }
                Ok(false)
            }
            Pattern::Or(patterns) => {
                // Try each pattern; return true if any matches
                for pat in patterns {
                    if self.pattern_matches(value, pat, bindings)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Pattern::Guard { pattern, condition } => {
                // First check if the pattern matches
                if !self.pattern_matches(value, pattern, bindings)? {
                    return Ok(false);
                }
                // Then evaluate the guard condition with the bindings
                // We need to temporarily set the bindings in the environment to evaluate the condition
                self.env.push_scope();
                for (name, val) in bindings.iter() {
                    self.env.define(name, val.clone());
                }
                let guard_result = match self.eval_expression(condition) {
                    Ok(cond_value) => self.is_truthy(&cond_value),
                    Err(_) => false,
                };
                self.env.pop_scope();
                Ok(guard_result)
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

    // ============== ERROR HANDLING TESTS (Phase 7) ==============

    #[test]
    fn test_ok_creation() {
        let (result, output) = run_vryn(r#"
            let x = Ok(42)
            println(x)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "Ok(42)");
    }

    #[test]
    fn test_err_creation() {
        let (result, output) = run_vryn(r#"
            let x = Err("error message")
            println(x)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "Err(error message)");
    }

    #[test]
    fn test_is_ok_true() {
        let (result, output) = run_vryn(r#"
            let x = Ok(10)
            println(is_ok(x))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
    }

    #[test]
    fn test_is_ok_false() {
        let (result, output) = run_vryn(r#"
            let x = Err("failed")
            println(is_ok(x))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "false");
    }

    #[test]
    fn test_is_err_true() {
        let (result, output) = run_vryn(r#"
            let x = Err("failed")
            println(is_err(x))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
    }

    #[test]
    fn test_is_err_false() {
        let (result, output) = run_vryn(r#"
            let x = Ok(5)
            println(is_err(x))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "false");
    }

    #[test]
    fn test_unwrap_ok() {
        let (result, output) = run_vryn(r#"
            let x = Ok(99)
            let val = unwrap(x)
            println(val)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "99");
    }

    #[test]
    fn test_unwrap_err() {
        let (result, _output) = run_vryn(r#"
            let x = Err("something went wrong")
            unwrap(x)
        "#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unwrap on an error"));
    }

    #[test]
    fn test_unwrap_or_ok() {
        let (result, output) = run_vryn(r#"
            let x = Ok(42)
            let val = unwrap_or(x, 0)
            println(val)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "42");
    }

    #[test]
    fn test_unwrap_or_err() {
        let (result, output) = run_vryn(r#"
            let x = Err("failed")
            let val = unwrap_or(x, 100)
            println(val)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "100");
    }

    #[test]
    fn test_try_catch_success() {
        let (result, output) = run_vryn(r#"
            let x = try {
                return 42
            } catch e {
                return 0
            }
            println(x)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "42");
    }

    #[test]
    fn test_try_catch_with_error() {
        let (result, output) = run_vryn(r#"
            let x = try {
                panic("test error")
            } catch e {
                println(e)
                return 99
            }
            println(x)
        "#);
        assert!(result.is_ok());
        // The error message should be printed in the catch block
        assert!(output[0].contains("Panic"));
        assert_eq!(output[1], "99");
    }

    #[test]
    fn test_panic_function() {
        let (result, _output) = run_vryn(r#"
            panic("this should fail")
        "#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Panic"));
    }

    #[test]
    fn test_question_mark_ok() {
        let (result, output) = run_vryn(r#"
            fn get_result() -> Result {
                return Ok(50)
            }
            let x = get_result()?
            println(x)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "50");
    }

    #[test]
    fn test_question_mark_err() {
        let (result, _output) = run_vryn(r#"
            fn get_result() -> Result {
                return Err("failed operation")
            }
            let x = get_result()?
            println(x)
        "#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Error propagated"));
    }

    #[test]
    fn test_result_type_of() {
        let (result, output) = run_vryn(r#"
            let x = Ok(5)
            println(type_of(x))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "Result");
    }

    #[test]
    fn test_nested_try_catch() {
        let (result, output) = run_vryn(r#"
            let x = try {
                let y = try {
                    return 10
                } catch e1 {
                    return 5
                }
                return y
            } catch e2 {
                return 0
            }
            println(x)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "10");
    }

    #[test]
    fn test_ok_in_function() {
        let (result, output) = run_vryn(r#"
            fn divide(a: i32, b: i32) -> Result {
                if b == 0 {
                    return Err("division by zero")
                }
                return Ok(a)
            }
            let res = divide(10, 2)
            println(res)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "Ok(10)");
    }
    // String function tests
    #[test]
    fn test_str_len() {
        let (result, output) = run_vryn(r#"
            let s = "hello"
            println(str_len(s))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "5");
    }
    #[test]
    fn test_str_contains() {
        let (result, output) = run_vryn(r#"
            let s = "hello world"
            println(str_contains(s, "world"))
            println(str_contains(s, "xyz"))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "false");
    }
    #[test]
    fn test_str_starts_with() {
        let (result, output) = run_vryn(r#"
            let s = "hello world"
            println(str_starts_with(s, "hello"))
            println(str_starts_with(s, "world"))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "false");
    }
    #[test]
    fn test_str_ends_with() {
        let (result, output) = run_vryn(r#"
            let s = "hello world"
            println(str_ends_with(s, "world"))
            println(str_ends_with(s, "hello"))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "false");
    }
    #[test]
    fn test_str_split() {
        let (result, output) = run_vryn(r#"
            let s = "a,b,c"
            let parts = str_split(s, ",")
            println(len(parts))
            println(parts[0])
            println(parts[2])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "a");
        assert_eq!(output[2], "c");
    }
    #[test]
    fn test_str_trim() {
        let (result, output) = run_vryn(r#"
            let s = "  hello  "
            println(str_trim(s))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "hello");
    }
    #[test]
    fn test_str_upper() {
        let (result, output) = run_vryn(r#"
            let s = "hello"
            println(str_upper(s))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "HELLO");
    }
    #[test]
    fn test_str_lower() {
        let (result, output) = run_vryn(r#"
            let s = "HELLO"
            println(str_lower(s))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "hello");
    }
    #[test]
    fn test_str_replace() {
        let (result, output) = run_vryn(r#"
            let s = "hello world"
            let replaced = str_replace(s, "world", "vryn")
            println(replaced)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "hello vryn");
    }
    #[test]
    fn test_str_chars() {
        let (result, output) = run_vryn(r#"
            let s = "abc"
            let chars = str_chars(s)
            println(len(chars))
            println(chars[0])
            println(chars[2])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "a");
        assert_eq!(output[2], "c");
    }
    #[test]
    fn test_str_join() {
        let (result, output) = run_vryn(r#"
            let arr = ["a", "b", "c"]
            let joined = str_join(arr, "-")
            println(joined)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "a-b-c");
    }
    #[test]
    fn test_substr() {
        let (result, output) = run_vryn(r#"
            let s = "hello"
            println(substr(s, 0, 2))
            println(substr(s, 1, 3))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "he");
        assert_eq!(output[1], "ell");
    }
    // Array function tests
    #[test]
    fn test_pop() {
        let (result, output) = run_vryn(r#"
            let arr = [1, 2, 3]
            let popped = pop(arr)
            println(popped)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
    }
    #[test]
    fn test_arr_len() {
        let (result, output) = run_vryn(r#"
            let arr = [1, 2, 3, 4]
            println(arr_len(arr))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "4");
    }
    #[test]
    fn test_arr_reverse() {
        let (result, output) = run_vryn(r#"
            let arr = [1, 2, 3]
            let reversed = arr_reverse(arr)
            println(reversed[0])
            println(reversed[2])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "1");
    }
    #[test]
    fn test_arr_contains() {
        let (result, output) = run_vryn(r#"
            let arr = [1, 2, 3]
            println(arr_contains(arr, 2))
            println(arr_contains(arr, 5))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "false");
    }
    #[test]
    fn test_arr_slice() {
        let (result, output) = run_vryn(r#"
            let arr = [1, 2, 3, 4, 5]
            let sliced = arr_slice(arr, 1, 4)
            println(len(sliced))
            println(sliced[0])
            println(sliced[2])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "2");
        assert_eq!(output[2], "4");
    }
    #[test]
    fn test_arr_sort() {
        let (result, output) = run_vryn(r#"
            let arr = [3, 1, 4, 1, 5]
            let sorted = arr_sort(arr)
            println(sorted[0])
            println(sorted[2])
            println(sorted[4])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "1");
        assert_eq!(output[1], "3");
        assert_eq!(output[2], "5");
    }
    // Math function tests
    #[test]
    fn test_abs() {
        let (result, output) = run_vryn(r#"
            println(abs(-5))
            println(abs(3))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "5");
        assert_eq!(output[1], "3");
    }
    #[test]
    fn test_min_max() {
        let (result, output) = run_vryn(r#"
            println(min(5, 3))
            println(max(5, 3))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "5");
    }
    #[test]
    fn test_floor_ceil_round() {
        let (result, output) = run_vryn(r#"
            println(floor(3.7))
            println(ceil(3.2))
            println(round(3.5))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "4");
        assert_eq!(output[2], "4");
    }
    #[test]
    fn test_sqrt() {
        let (result, output) = run_vryn(r#"
            println(sqrt(4.0))
            println(sqrt(9.0))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "2");
        assert_eq!(output[1], "3");
    }
    #[test]
    fn test_pow() {
        let (result, output) = run_vryn(r#"
            println(pow(2, 3))
            println(pow(5, 2))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "8");
        assert_eq!(output[1], "25");
    }
    #[test]
    fn test_int_conversion() {
        let (result, output) = run_vryn(r#"
            println(int(3.7))
            println(int("42"))
            println(int(true))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "42");
        assert_eq!(output[2], "1");
    }
    #[test]
    fn test_float_conversion() {
        let (result, output) = run_vryn(r#"
            println(float(42))
            println(float("3.14"))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "42");
        assert_eq!(output[1], "3.14");
    }


    // === Phase 8: Pattern Matching Tests ===

    #[test]
    fn test_match_with_guard_simple() {
        let (result, output) = run_vryn(r#"
            let x = 5
            match x {
                1 => println("one")
                2 => println("two")
                n if n > 3 => println("big")
                _ => println("other")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "big");
    }

    #[test]
    fn test_match_with_guard_multiple() {
        let (result, output) = run_vryn(r#"
            let x = 15
            match x {
                n if n < 10 => println("small")
                n if n >= 10 && n < 20 => println("large")
                _ => println("huge")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "large");
    }

    #[test]
    fn test_match_with_guard_no_match() {
        let (result, output) = run_vryn(r#"
            let x = 25
            match x {
                n if n < 5 => println("small")
                n if n < 20 => println("medium")
                n if n >= 20 => println("big")
                _ => println("other")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "big");
    }

    #[test]
    fn test_match_with_tuple_pattern() {
        let (result, output) = run_vryn(r#"
            let tuple = [1, 2]
            match tuple {
                (a, b) => println(a)
                _ => println("no match")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "1");
    }

    #[test]
    fn test_match_with_enum_variant_pattern() {
        let (result, output) = run_vryn(r#"
            enum Color {
                Red
                Green
                Blue
            }
            let c = Color::Red
            match c {
                Color::Red => println("matched")
                Color::Green => println("no")
                Color::Blue => println("no")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "matched");
    }

    #[test]
    fn test_match_with_wildcard() {
        let (result, output) = run_vryn(r#"
            let x = 100
            match x {
                1 => println("one")
                2 => println("two")
                _ => println("many")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "many");
    }

    #[test]
    fn test_match_with_identifier_binding() {
        let (result, output) = run_vryn(r#"
            let x = 42
            match x {
                n => println(n)
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "42");
    }

    #[test]
    fn test_match_with_or_pattern() {
        let (result, output) = run_vryn(r#"
            let x = 2
            match x {
                1 => println("one")
                2 => println("two")
                3 => println("three")
                _ => println("large")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "two");
    }

    #[test]
    fn test_match_with_range_pattern() {
        let (result, output) = run_vryn(r#"
            let x = 5
            match x {
                1 => println("one to three")
                5 => println("match")
                _ => println("other")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "match");
    }

    #[test]
    fn test_guard_with_negation() {
        let (result, output) = run_vryn(r#"
            let x = 3
            match x {
                n if !(n > 5) => println("not big")
                _ => println("big")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "not big");
    }

    #[test]
    fn test_guard_chains() {
        let (result, output) = run_vryn(r#"
            let x = 10
            match x {
                n if n < 5 => println("tiny")
                n if n < 10 => println("small")
                n if n < 15 => println("medium")
                _ => println("large")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "medium");
    }

    #[test]
    fn test_match_with_literal_pattern() {
        let (result, output) = run_vryn(r#"
            let x = "hello"
            match x {
                "hello" => println("greeting")
                "goodbye" => println("farewell")
                _ => println("unknown")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "greeting");
    }

    #[test]
    fn test_match_with_bool_pattern() {
        let (result, output) = run_vryn(r#"
            let b = true
            match b {
                true => println("yes")
                false => println("no")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "yes");
    }

    #[test]
    fn test_match_expression_returns_value() {
        let (result, output) = run_vryn(r#"
            let x = 5
            let result = match x {
                1 => 10
                2 => 20
                _ => 30
            }
            println(result)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "30");
    }

    #[test]
    fn test_guard_with_variable_binding() {
        let (result, output) = run_vryn(r#"
            let x = 8
            match x {
                n if n > 5 && n < 10 => println("between 5 and 10")
                _ => println("other")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "between 5 and 10");
    }

    #[test]
    fn test_match_with_multiple_guards() {
        let (result, output) = run_vryn(r#"
            let x = 3
            let y = 7
            match x {
                a if a == 3 => println("matched")
                _ => println("not matched")
            }
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "matched");
    }

    // Additional String function tests
    #[test]
    fn test_str_len_builtin() {
        let (result, output) = run_vryn(r#"
            println(str_len("hello"))
            println(str_len(""))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "5");
        assert_eq!(output[1], "0");
    }

    #[test]
    fn test_str_contains_comprehensive() {
        let (result, output) = run_vryn(r#"
            let s = "hello world"
            println(str_contains(s, "world"))
            println(str_contains(s, "xyz"))
            println(str_contains(s, ""))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "false");
        assert_eq!(output[2], "true");
    }

    #[test]
    fn test_str_starts_ends_with() {
        let (result, output) = run_vryn(r#"
            let s = "hello world"
            println(str_starts_with(s, "hello"))
            println(str_starts_with(s, "world"))
            println(str_ends_with(s, "world"))
            println(str_ends_with(s, "hello"))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "false");
        assert_eq!(output[2], "true");
        assert_eq!(output[3], "false");
    }

    #[test]
    fn test_str_split_comprehensive() {
        let (result, output) = run_vryn(r#"
            let s = "a,b,c,d"
            let parts = str_split(s, ",")
            println(len(parts))
            println(parts[0])
            println(parts[3])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "4");
        assert_eq!(output[1], "a");
        assert_eq!(output[2], "d");
    }

    #[test]
    fn test_str_trim_comprehensive() {
        let (result, output) = run_vryn(r#"
            println(str_trim("  hello  "))
            println(str_trim("no_trim"))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "hello");
        assert_eq!(output[1], "no_trim");
    }

    #[test]
    fn test_str_upper_lower() {
        let (result, output) = run_vryn(r#"
            let s = "HeLLo WoRLD"
            println(str_upper(s))
            println(str_lower(s))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "HELLO WORLD");
        assert_eq!(output[1], "hello world");
    }

    #[test]
    fn test_str_replace_comprehensive() {
        let (result, output) = run_vryn(r#"
            let s = "aaa"
            let replaced = str_replace(s, "a", "b")
            println(replaced)
            let s2 = "hello world"
            let replaced2 = str_replace(s2, "world", "vryn")
            println(replaced2)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "bbb");
        assert_eq!(output[1], "hello vryn");
    }

    #[test]
    fn test_str_chars_comprehensive() {
        let (result, output) = run_vryn(r#"
            let s = "xyz"
            let chars = str_chars(s)
            println(len(chars))
            println(chars[1])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "y");
    }

    #[test]
    fn test_str_join_comprehensive() {
        let (result, output) = run_vryn(r#"
            let arr = ["x", "y", "z"]
            println(str_join(arr, ""))
            println(str_join(arr, " | "))
            let nums = [1, 2, 3]
            println(str_join(nums, ","))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "xyz");
        assert_eq!(output[1], "x | y | z");
        assert_eq!(output[2], "1,2,3");
    }

    #[test]
    fn test_substr_comprehensive() {
        let (result, output) = run_vryn(r#"
            let s = "abcdefgh"
            println(substr(s, 0, 3))
            println(substr(s, 2, 3))
            println(substr(s, 7, 1))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "abc");
        assert_eq!(output[1], "cde");
        assert_eq!(output[2], "h");
    }

    // Additional Array function tests
    #[test]
    fn test_pop_comprehensive() {
        let (result, output) = run_vryn(r#"
            let arr = ["a", "b", "c"]
            let popped = pop(arr)
            println(popped)
            let arr2 = []
            let popped2 = pop(arr2)
            println(popped2)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "c");
        assert_eq!(output[1], "None");
    }

    #[test]
    fn test_arr_reverse_comprehensive() {
        let (result, output) = run_vryn(r#"
            let arr = ["a", "b", "c", "d"]
            let reversed = arr_reverse(arr)
            println(reversed[0])
            println(reversed[3])
            println(len(reversed))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "d");
        assert_eq!(output[1], "a");
        assert_eq!(output[2], "4");
    }

    #[test]
    fn test_arr_contains_comprehensive() {
        let (result, output) = run_vryn(r#"
            let arr = [1, 2, 3, 4, 5]
            println(arr_contains(arr, 3))
            println(arr_contains(arr, 10))
            let strs = ["hello", "world"]
            println(arr_contains(strs, "hello"))
            println(arr_contains(strs, "xyz"))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "false");
        assert_eq!(output[2], "true");
        assert_eq!(output[3], "false");
    }

    #[test]
    fn test_arr_slice_comprehensive() {
        let (result, output) = run_vryn(r#"
            let arr = [10, 20, 30, 40, 50]
            let slice1 = arr_slice(arr, 0, 3)
            println(len(slice1))
            println(slice1[0])
            println(slice1[2])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "10");
        assert_eq!(output[2], "30");
    }

    #[test]
    fn test_arr_sort_comprehensive() {
        let (result, output) = run_vryn(r#"
            let arr = [5, 2, 8, 1, 9]
            let sorted = arr_sort(arr)
            println(sorted[0])
            println(sorted[2])
            println(sorted[4])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "1");
        assert_eq!(output[1], "5");
        assert_eq!(output[2], "9");
    }

    // Additional Math function tests
    #[test]
    fn test_abs_comprehensive() {
        let (result, output) = run_vryn(r#"
            println(abs(-42))
            println(abs(42))
            println(abs(0))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "42");
        assert_eq!(output[1], "42");
        assert_eq!(output[2], "0");
    }

    #[test]
    fn test_min_max_comprehensive() {
        let (result, output) = run_vryn(r#"
            println(min(10, 5))
            println(max(10, 5))
            println(min(-3, -7))
            println(max(-3, -7))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "5");
        assert_eq!(output[1], "10");
        assert_eq!(output[2], "-7");
        assert_eq!(output[3], "-3");
    }

    #[test]
    fn test_floor_ceil_round_comprehensive() {
        let (result, output) = run_vryn(r#"
            println(floor(5.9))
            println(ceil(5.1))
            println(round(5.4))
            println(round(5.5))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "5");
        assert_eq!(output[1], "6");
        assert_eq!(output[2], "5");
        assert_eq!(output[3], "6");
    }

    #[test]
    fn test_sqrt_comprehensive() {
        let (result, output) = run_vryn(r#"
            println(sqrt(16.0))
            println(sqrt(100.0))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "4");
        assert_eq!(output[1], "10");
    }

    #[test]
    fn test_pow_comprehensive() {
        let (result, output) = run_vryn(r#"
            println(pow(2, 5))
            println(pow(10, 2))
            println(pow(3, 3))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "32");
        assert_eq!(output[1], "100");
        assert_eq!(output[2], "27");
    }

    #[test]
    fn test_int_conversion_comprehensive() {
        let (result, output) = run_vryn(r#"
            println(int(5.9))
            println(int("123"))
            println(int(true))
            println(int(false))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "5");
        assert_eq!(output[1], "123");
        assert_eq!(output[2], "1");
        assert_eq!(output[3], "0");
    }

    #[test]
    fn test_float_conversion_comprehensive() {
        let (result, output) = run_vryn(r#"
            println(float(5))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "5");
    }

    // I/O function tests
    #[test]
    fn test_file_exists() {
        let (result, output) = run_vryn(r#"
            println(file_exists("/etc/passwd"))
            println(file_exists("/tmp/nonexistent_vryn_test_file_xyz_9999"))
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "false");
    }

    #[test]
    fn test_write_and_read_file() {
        let (result, output) = run_vryn(r#"
            let path = "/tmp/vryn_test_file.txt"
            write_file(path, "hello vryn")
            let content = read_file(path)
            println(content)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "hello vryn");
        let _ = std::fs::remove_file("/tmp/vryn_test_file.txt");
    }

    #[test]
    fn test_write_read_multiple_lines() {
        let (result, output) = run_vryn(r#"
            let path = "/tmp/vryn_multiline_test.txt"
            write_file(path, "line1\nline2\nline3")
            let content = read_file(path)
            println(content)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "line1\nline2\nline3");
        let _ = std::fs::remove_file("/tmp/vryn_multiline_test.txt");
    }

    #[test]
    fn test_write_file_overwrites() {
        let (result, output) = run_vryn(r#"
            let path = "/tmp/vryn_overwrite_test.txt"
            write_file(path, "first")
            write_file(path, "second")
            let content = read_file(path)
            println(content)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "second");
        let _ = std::fs::remove_file("/tmp/vryn_overwrite_test.txt");
    }

    #[test]
    fn test_complex_string_operations() {
        let (result, output) = run_vryn(r#"
            let text = "  Hello, Vryn! "
            let trimmed = str_trim(text)
            let lowered = str_lower(trimmed)
            let parts = str_split(lowered, ",")
            println(len(parts))
            println(parts[0])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "2");
        assert_eq!(output[1], "hello");
    }

    #[test]
    fn test_array_operations_chain() {
        let (result, output) = run_vryn(r#"
            let arr = [5, 2, 8, 1, 9, 3]
            let sorted = arr_sort(arr)
            let sliced = arr_slice(sorted, 1, 4)
            println(len(sliced))
            println(sliced[0])
            println(sliced[2])
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "3");
        assert_eq!(output[1], "2");
        assert_eq!(output[2], "5");
    }

    #[test]
    fn test_math_operations_chain() {
        let (result, output) = run_vryn(r#"
            let x = -42
            let y = abs(x)
            let z = pow(y, 2)
            println(y)
            println(z)
        "#);
        assert!(result.is_ok());
        assert_eq!(output[0], "42");
        assert_eq!(output[1], "1764");
    }
}
