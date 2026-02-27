use std::collections::HashMap;
use std::fmt;
use crate::parser::ast::*;

// ============================================================================
// VrynType enum - Represents all possible types in Vryn
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum VrynType {
    /// Primitive integer type (i64)
    Int,
    /// Primitive float type (f64)
    Float,
    /// Primitive string type
    Str,
    /// Primitive boolean type
    Bool,
    /// Void/None type (unit type)
    Void,
    /// Array type with element type
    Array(Box<VrynType>),
    /// Function type with parameter types and return type
    Function {
        params: Vec<VrynType>,
        return_type: Box<VrynType>,
    },
    /// Struct type with field types
    Struct {
        name: String,
        fields: HashMap<String, VrynType>,
    },
    /// Enum type with variants
    Enum {
        name: String,
    },
    /// Any type (for unresolved/unannotated variables)
    Any,
    /// Unknown type (used internally for error recovery)
    Unknown,
    /// Error type (indicates a type error occurred)
    Error,
}

impl fmt::Display for VrynType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VrynType::Int => write!(f, "i32"),
            VrynType::Float => write!(f, "f64"),
            VrynType::Str => write!(f, "str"),
            VrynType::Bool => write!(f, "bool"),
            VrynType::Void => write!(f, "void"),
            VrynType::Array(elem_type) => write!(f, "[{}]", elem_type),
            VrynType::Function { params, return_type } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            VrynType::Struct { name, .. } => write!(f, "struct {}", name),
            VrynType::Enum { name } => write!(f, "enum {}", name),
            VrynType::Any => write!(f, "?"),
            VrynType::Unknown => write!(f, "<unknown>"),
            VrynType::Error => write!(f, "<error>"),
        }
    }
}

impl VrynType {
    /// Check if two types are compatible for assignment
    pub fn is_compatible_with(&self, other: &VrynType) -> bool {
        match (self, other) {
            // Exact match
            (a, b) if a == b => true,
            // Any type is compatible with anything
            (VrynType::Any, _) | (_, VrynType::Any) => true,
            // Error type is compatible with anything
            (VrynType::Error, _) | (_, VrynType::Error) => true,
            // Arrays must have compatible element types
            (VrynType::Array(a), VrynType::Array(b)) => a.is_compatible_with(b),
            // Otherwise not compatible
            _ => false,
        }
    }

    /// Check if type can be converted to another type
    pub fn can_convert_to(&self, target: &VrynType) -> bool {
        self.is_compatible_with(target)
    }
}

// ============================================================================
// TypeError - Type error representation with location and message
// ============================================================================

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub line: Option<usize>,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(line) = self.line {
            write!(f, "line {}: {}", line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl TypeError {
    pub fn new(message: String) -> Self {
        TypeError { message, line: None }
    }

    pub fn with_line(message: String, line: usize) -> Self {
        TypeError { message, line: Some(line) }
    }
}

// ============================================================================
// TypeEnv - Symbol table with nested scopes
// ============================================================================

#[derive(Debug, Clone)]
struct VarInfo {
    type_: VrynType,
    mutable: bool,
}

pub struct TypeEnv {
    scopes: Vec<HashMap<String, VarInfo>>,
    functions: HashMap<String, (Vec<VrynType>, VrynType)>,
    structs: HashMap<String, HashMap<String, VrynType>>,
    enums: HashMap<String, Vec<String>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = TypeEnv {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
        };
        // Add built-in functions
        env.define_builtin_functions();
        env
    }

    fn define_builtin_functions(&mut self) {
        // println(str) -> void
        self.functions.insert(
            "println".to_string(),
            (vec![VrynType::Str], VrynType::Void),
        );
        // print(str) -> void
        self.functions.insert(
            "print".to_string(),
            (vec![VrynType::Str], VrynType::Void),
        );
        // len(array or str) -> i32
        self.functions.insert(
            "len".to_string(),
            (vec![VrynType::Any], VrynType::Int),
        );
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define(&mut self, name: String, type_: VrynType, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, VarInfo { type_, mutable });
        }
    }

    pub fn lookup(&self, name: &str) -> Option<VrynType> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.type_.clone());
            }
        }
        None
    }

    pub fn is_mutable(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return info.mutable;
            }
        }
        false
    }

    pub fn define_function(&mut self, name: String, params: Vec<VrynType>, return_type: VrynType) {
        self.functions.insert(name, (params, return_type));
    }

    pub fn lookup_function(&self, name: &str) -> Option<(Vec<VrynType>, VrynType)> {
        self.functions.get(name).cloned()
    }

    pub fn define_struct(&mut self, name: String, fields: HashMap<String, VrynType>) {
        self.structs.insert(name, fields);
    }

    pub fn lookup_struct(&self, name: &str) -> Option<HashMap<String, VrynType>> {
        self.structs.get(name).cloned()
    }

    pub fn define_enum(&mut self, name: String, variants: Vec<String>) {
        self.enums.insert(name, variants);
    }

    pub fn lookup_enum(&self, name: &str) -> Option<Vec<String>> {
        self.enums.get(name).cloned()
    }
}

// ============================================================================
// TypeChecker - Main type checking logic
// ============================================================================

pub struct TypeChecker {
    env: TypeEnv,
    errors: Vec<TypeError>,
    return_type_stack: Vec<VrynType>,
    in_loop: bool,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            env: TypeEnv::new(),
            errors: Vec::new(),
            return_type_stack: Vec::new(),
            in_loop: false,
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Vec<TypeError> {
        // First pass: collect definitions
        for stmt in &program.statements {
            if let Err(e) = self.collect_definitions(stmt) {
                self.errors.push(e);
            }
        }

        // Second pass: type check statements
        for stmt in &program.statements {
            if let Err(e) = self.check_statement(stmt) {
                self.errors.push(e);
            }
        }

        self.errors.clone()
    }

    /// First pass: collect all function/struct/enum definitions
    fn collect_definitions(&mut self, stmt: &Statement) -> Result<(), TypeError> {
        match stmt {
            Statement::Function { name, params, return_type, .. } => {
                let param_types: Vec<VrynType> = params
                    .iter()
                    .map(|p| self.parse_type_name(&p.type_name))
                    .collect();

                let ret_type = return_type
                    .as_ref()
                    .map(|t| self.parse_type_name(t))
                    .unwrap_or(VrynType::Void);

                self.env.define_function(name.clone(), param_types, ret_type);
                Ok(())
            }
            Statement::Struct { name, fields } => {
                let mut field_map = HashMap::new();
                for field in fields {
                    let type_ = self.parse_type_name(&field.type_name);
                    field_map.insert(field.name.clone(), type_);
                }
                self.env.define_struct(name.clone(), field_map);
                Ok(())
            }
            Statement::Enum { name, variants } => {
                let variant_names: Vec<String> = variants.iter().map(|v| v.name.clone()).collect();
                self.env.define_enum(name.clone(), variant_names);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn check_statement(&mut self, stmt: &Statement) -> Result<(), TypeError> {
        match stmt {
            Statement::Let {
                name,
                mutable,
                type_ann,
                value,
            } => {
                let value_type = self.infer_expr(value)?;

                let var_type = if let Some(ann) = type_ann {
                    let ann_type = self.parse_type_name(ann);
                    if !value_type.is_compatible_with(&ann_type) {
                        return Err(TypeError::new(format!(
                            "Type mismatch: expected {}, got {}",
                            ann_type, value_type
                        )));
                    }
                    ann_type
                } else {
                    value_type
                };

                self.env.define(name.clone(), var_type, *mutable);
                Ok(())
            }

            Statement::Function {
                name: _,
                params,
                return_type,
                body,
            } => {
                self.env.push_scope();

                let _param_types: Vec<VrynType> = params
                    .iter()
                    .map(|p| {
                        let type_ = self.parse_type_name(&p.type_name);
                        self.env.define(p.name.clone(), type_.clone(), false);
                        type_
                    })
                    .collect();

                let expected_ret = return_type
                    .as_ref()
                    .map(|t| self.parse_type_name(t))
                    .unwrap_or(VrynType::Void);

                self.return_type_stack.push(expected_ret.clone());

                for stmt in body {
                    if let Err(e) = self.check_statement(stmt) {
                        self.errors.push(e);
                    }
                }

                self.return_type_stack.pop();
                self.env.pop_scope();
                Ok(())
            }

            Statement::Struct { .. } => Ok(()), // Already collected in first pass

            Statement::Enum { .. } => Ok(()), // Already collected in first pass

            Statement::Trait { .. } => Ok(()), // Trait definitions don't need type checking yet

            Statement::Impl { .. } => Ok(()), // Impl blocks are checked at usage time

            Statement::Expression(expr) => {
                self.infer_expr(expr)?;
                Ok(())
            }

            Statement::Return(Some(expr)) => {
                let expr_type = self.infer_expr(expr)?;
                if let Some(expected) = self.return_type_stack.last() {
                    if !expr_type.is_compatible_with(expected) {
                        return Err(TypeError::new(format!(
                            "Return type mismatch: expected {}, got {}",
                            expected, expr_type
                        )));
                    }
                }
                Ok(())
            }

            Statement::Return(None) => {
                if let Some(expected) = self.return_type_stack.last() {
                    if !matches!(expected, VrynType::Void) {
                        return Err(TypeError::new(format!(
                            "Return type mismatch: expected {}, got void",
                            expected
                        )));
                    }
                }
                Ok(())
            }

            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let cond_type = self.infer_expr(condition)?;
                if !matches!(cond_type, VrynType::Bool | VrynType::Any | VrynType::Error) {
                    return Err(TypeError::new(format!(
                        "If condition must be bool, got {}",
                        cond_type
                    )));
                }

                self.env.push_scope();
                for stmt in then_body {
                    if let Err(e) = self.check_statement(stmt) {
                        self.errors.push(e);
                    }
                }
                self.env.pop_scope();

                if let Some(else_stmts) = else_body {
                    self.env.push_scope();
                    for stmt in else_stmts {
                        if let Err(e) = self.check_statement(stmt) {
                            self.errors.push(e);
                        }
                    }
                    self.env.pop_scope();
                }

                Ok(())
            }

            Statement::While { condition, body } => {
                let cond_type = self.infer_expr(condition)?;
                if !matches!(cond_type, VrynType::Bool | VrynType::Any | VrynType::Error) {
                    return Err(TypeError::new(format!(
                        "While condition must be bool, got {}",
                        cond_type
                    )));
                }

                let was_in_loop = self.in_loop;
                self.in_loop = true;

                self.env.push_scope();
                for stmt in body {
                    if let Err(e) = self.check_statement(stmt) {
                        self.errors.push(e);
                    }
                }
                self.env.pop_scope();

                self.in_loop = was_in_loop;
                Ok(())
            }

            Statement::For {
                var,
                iterable,
                body,
            } => {
                let iter_type = self.infer_expr(iterable)?;

                self.env.push_scope();

                // Infer element type from iterable
                match iter_type {
                    VrynType::Array(elem_type) => {
                        self.env.define(var.clone(), *elem_type, false);
                    }
                    VrynType::Str => {
                        self.env.define(var.clone(), VrynType::Str, false);
                    }
                    VrynType::Any => {
                        self.env.define(var.clone(), VrynType::Any, false);
                    }
                    VrynType::Error => {
                        self.env.define(var.clone(), VrynType::Error, false);
                    }
                    _ => {
                        return Err(TypeError::new(format!(
                            "For loop iterable must be array or string, got {}",
                            iter_type
                        )));
                    }
                }

                let was_in_loop = self.in_loop;
                self.in_loop = true;

                for stmt in body {
                    if let Err(e) = self.check_statement(stmt) {
                        self.errors.push(e);
                    }
                }

                self.in_loop = was_in_loop;
                self.env.pop_scope();
                Ok(())
            }

            Statement::Break => {
                if !self.in_loop {
                    return Err(TypeError::new(
                        "break statement outside of loop".to_string(),
                    ));
                }
                Ok(())
            }

            Statement::Continue => {
                if !self.in_loop {
                    return Err(TypeError::new(
                        "continue statement outside of loop".to_string(),
                    ));
                }
                Ok(())
            }

            Statement::WhileLet { pattern: _, expr, body } => {
                let _expr_type = self.infer_expr(expr)?;
                let was_in_loop = self.in_loop;
                self.in_loop = true;

                self.env.push_scope();
                for stmt in body {
                    if let Err(e) = self.check_statement(stmt) {
                        self.errors.push(e);
                    }
                }
                self.env.pop_scope();

                self.in_loop = was_in_loop;
                Ok(())
            }

            Statement::IfLet { pattern: _, expr, then_body, else_body } => {
                let _expr_type = self.infer_expr(expr)?;
                for stmt in then_body {
                    self.check_statement(stmt)?;
                }
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        self.check_statement(stmt)?;
                    }
                }
                Ok(())
            }

            Statement::Import { path: _, alias: _ } => {
                // Import statements are checked during first pass
                Ok(())
            }
        }
    }

    pub fn infer_expr(&mut self, expr: &Expression) -> Result<VrynType, TypeError> {
        match expr {
            Expression::IntLiteral(_) => Ok(VrynType::Int),
            Expression::FloatLiteral(_) => Ok(VrynType::Float),
            Expression::StringLiteral(_) => Ok(VrynType::Str),
            Expression::BoolLiteral(_) => Ok(VrynType::Bool),

            Expression::Identifier(name) => {
                self.env.lookup(name).ok_or_else(|| {
                    TypeError::new(format!("Undefined variable: {}", name))
                })
            }

            Expression::BinaryOp { left, op, right } => {
                let left_type = self.infer_expr(left)?;
                let right_type = self.infer_expr(right)?;

                self.check_binary_op(&left_type, op, &right_type)
            }

            Expression::UnaryOp { op, operand } => {
                let operand_type = self.infer_expr(operand)?;
                self.check_unary_op(op, &operand_type)
            }

            Expression::Call { function, args } => {
                let func_name = match function.as_ref() {
                    Expression::Identifier(name) => name.clone(),
                    _ => {
                        // Complex function expression - for now just infer as Any
                        return Ok(VrynType::Any);
                    }
                };

                // Look up function signature
                if let Some((param_types, return_type)) = self.env.lookup_function(&func_name) {
                    if args.len() != param_types.len() {
                        return Err(TypeError::new(format!(
                            "Function {} expects {} arguments, got {}",
                            func_name,
                            param_types.len(),
                            args.len()
                        )));
                    }

                    for (arg, expected_type) in args.iter().zip(param_types.iter()) {
                        let arg_type = self.infer_expr(arg)?;
                        if !arg_type.is_compatible_with(expected_type) {
                            return Err(TypeError::new(format!(
                                "Argument type mismatch: expected {}, got {}",
                                expected_type, arg_type
                            )));
                        }
                    }

                    Ok(return_type)
                } else {
                    // Unknown function - return Any to avoid cascading errors
                    Err(TypeError::new(format!(
                        "Undefined function: {}",
                        func_name
                    )))
                }
            }

            Expression::MemberAccess { object, member } => {
                let obj_type = self.infer_expr(object)?;
                match obj_type {
                    VrynType::Struct { ref fields, .. } => {
                        fields.get(member).cloned().ok_or_else(|| {
                            TypeError::new(format!(
                                "Struct has no field named '{}'",
                                member
                            ))
                        })
                    }
                    VrynType::Any | VrynType::Error => Ok(VrynType::Any),
                    _ => Err(TypeError::new(format!(
                        "Cannot access member '{}' on type {}",
                        member, obj_type
                    ))),
                }
            }

            Expression::Index { object, index } => {
                let obj_type = self.infer_expr(object)?;
                let idx_type = self.infer_expr(index)?;

                if !matches!(idx_type, VrynType::Int | VrynType::Any | VrynType::Error) {
                    return Err(TypeError::new(format!(
                        "Array index must be int, got {}",
                        idx_type
                    )));
                }

                match obj_type {
                    VrynType::Array(elem_type) => Ok(*elem_type),
                    VrynType::Str => Ok(VrynType::Str), // Indexing string returns char-like
                    VrynType::Any | VrynType::Error => Ok(VrynType::Any),
                    _ => Err(TypeError::new(format!(
                        "Cannot index type {}",
                        obj_type
                    ))),
                }
            }

            Expression::Assign { target, value } => {
                let value_type = self.infer_expr(value)?;

                // Check if target is assignable
                match target.as_ref() {
                    Expression::Identifier(name) => {
                        if !self.env.is_mutable(name) {
                            return Err(TypeError::new(format!(
                                "Cannot assign to immutable variable '{}'",
                                name
                            )));
                        }

                        if let Some(target_type) = self.env.lookup(name) {
                            if !value_type.is_compatible_with(&target_type) {
                                return Err(TypeError::new(format!(
                                    "Type mismatch in assignment: expected {}, got {}",
                                    target_type, value_type
                                )));
                            }
                        }
                        Ok(value_type)
                    }
                    Expression::Index { .. } => {
                        // Array element assignment is allowed
                        Ok(value_type)
                    }
                    Expression::MemberAccess { .. } => {
                        // Struct field assignment is allowed
                        Ok(value_type)
                    }
                    _ => Err(TypeError::new(
                        "Invalid assignment target".to_string(),
                    )),
                }
            }

            Expression::Array(elements) => {
                if elements.is_empty() {
                    Ok(VrynType::Array(Box::new(VrynType::Any)))
                } else {
                    let first_type = self.infer_expr(&elements[0])?;

                    // Check all elements have compatible types
                    for elem in &elements[1..] {
                        let elem_type = self.infer_expr(elem)?;
                        if !elem_type.is_compatible_with(&first_type) {
                            return Err(TypeError::new(format!(
                                "Array element type mismatch: expected {}, got {}",
                                first_type, elem_type
                            )));
                        }
                    }

                    Ok(VrynType::Array(Box::new(first_type)))
                }
            }

            Expression::Pipe { left, right } => {
                let _left_type = self.infer_expr(left)?;

                // For pipe expressions, infer right type
                // The left result is implicitly passed to right
                self.infer_expr(right)
            }

            Expression::Range { start, end, .. } => {
                let start_type = self.infer_expr(start)?;
                let end_type = self.infer_expr(end)?;

                if !matches!(start_type, VrynType::Int | VrynType::Any | VrynType::Error) {
                    return Err(TypeError::new(format!(
                        "Range start must be int, got {}",
                        start_type
                    )));
                }
                if !matches!(end_type, VrynType::Int | VrynType::Any | VrynType::Error) {
                    return Err(TypeError::new(format!(
                        "Range end must be int, got {}",
                        end_type
                    )));
                }

                Ok(VrynType::Array(Box::new(VrynType::Int)))
            }

            Expression::Match { value, arms } => {
                let _value_type = self.infer_expr(value)?;

                // All arms should return compatible types
                let mut arm_types = Vec::new();
                for arm in arms {
                    let arm_type = self.infer_expr(&arm.body)?;
                    arm_types.push(arm_type);
                }

                if arm_types.is_empty() {
                    Ok(VrynType::Void)
                } else if arm_types.iter().all(|t| t == &arm_types[0]) {
                    Ok(arm_types[0].clone())
                } else {
                    Ok(VrynType::Any) // Mixed types default to Any
                }
            }

            Expression::Block(statements) => {
                self.env.push_scope();

                let mut last_type = VrynType::Void;

                for stmt in statements {
                    match stmt {
                        Statement::Expression(expr) => {
                            last_type = self.infer_expr(expr)?;
                        }
                        _ => {
                            self.check_statement(stmt)?;
                        }
                    }
                }

                self.env.pop_scope();
                Ok(last_type)
            }

            Expression::StructInit { name, fields } => {
                if let Some(struct_fields) = self.env.lookup_struct(name) {
                    for (field_name, field_expr) in fields {
                        let field_type = self.infer_expr(field_expr)?;

                        if let Some(expected_type) = struct_fields.get(field_name) {
                            if !field_type.is_compatible_with(expected_type) {
                                return Err(TypeError::new(format!(
                                    "Struct field '{}' type mismatch: expected {}, got {}",
                                    field_name, expected_type, field_type
                                )));
                            }
                        } else {
                            return Err(TypeError::new(format!(
                                "Struct '{}' has no field '{}'",
                                name, field_name
                            )));
                        }
                    }

                    Ok(VrynType::Struct {
                        name: name.clone(),
                        fields: struct_fields,
                    })
                } else {
                    Err(TypeError::new(format!("Undefined struct: {}", name)))
                }
            }

            Expression::Lambda { params: _, body } => {
                let _body_type = self.infer_expr(body)?;
                // For now, represent lambdas as functions with Any params/return
                Ok(VrynType::Function {
                    params: vec![VrynType::Any],
                    return_type: Box::new(VrynType::Any),
                })
            }

            Expression::TryCatch { try_body, catch_var: _, catch_body } => {
                self.env.push_scope();

                // Check try block
                let mut try_type = VrynType::Void;
                for stmt in try_body {
                    match stmt {
                        Statement::Expression(expr) => {
                            try_type = self.infer_expr(expr)?;
                        }
                        _ => {
                            self.check_statement(stmt)?;
                        }
                    }
                }

                // Check catch block
                let mut catch_type = VrynType::Void;
                for stmt in catch_body {
                    match stmt {
                        Statement::Expression(expr) => {
                            catch_type = self.infer_expr(expr)?;
                        }
                        _ => {
                            self.check_statement(stmt)?;
                        }
                    }
                }

                self.env.pop_scope();

                // Both branches should have compatible types
                if try_type == catch_type {
                    Ok(try_type)
                } else {
                    Ok(VrynType::Any)
                }
            }

            Expression::QuestionMark { expr } => {
                let _expr_type = self.infer_expr(expr)?;
                // Question mark operator expects a Result type and unwraps it
                // For now, we return Any to avoid strict type checking
                Ok(VrynType::Any)
            }

            Expression::MethodCall { object, method, args } => {
                // For now, just check that the object and args are valid
                let _obj_type = self.infer_expr(object)?;
                for arg in args {
                    let _arg_type = self.infer_expr(arg)?;
                }
                // Return Any since we don't have full type information for methods yet
                Ok(VrynType::Any)
            }

            Expression::Self_ => {
                // Self references are allowed in method contexts
                // For now, return Any type
                Ok(VrynType::Any)
            }
        }
    }

    fn check_binary_op(
        &mut self,
        left_type: &VrynType,
        op: &BinaryOperator,
        right_type: &VrynType,
    ) -> Result<VrynType, TypeError> {
        match op {
            BinaryOperator::Add | BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div | BinaryOperator::Mod => {
                match (left_type, right_type) {
                    (VrynType::Int, VrynType::Int) => Ok(VrynType::Int),
                    (VrynType::Float, VrynType::Float) => Ok(VrynType::Float),
                    (VrynType::Int, VrynType::Float) => Ok(VrynType::Float),
                    (VrynType::Float, VrynType::Int) => Ok(VrynType::Float),
                    (VrynType::Str, VrynType::Str) if matches!(op, BinaryOperator::Add) => {
                        Ok(VrynType::Str)
                    }
                    (VrynType::Any, _) | (_, VrynType::Any) => Ok(VrynType::Any),
                    (VrynType::Error, _) | (_, VrynType::Error) => Ok(VrynType::Error),
                    _ => Err(TypeError::new(format!(
                        "Invalid operands for {:?}: {} and {}",
                        op, left_type, right_type
                    ))),
                }
            }

            BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Less
            | BinaryOperator::Greater
            | BinaryOperator::LessEq
            | BinaryOperator::GreaterEq => {
                match (left_type, right_type) {
                    (VrynType::Int, VrynType::Int) => Ok(VrynType::Bool),
                    (VrynType::Float, VrynType::Float) => Ok(VrynType::Bool),
                    (VrynType::Int, VrynType::Float) => Ok(VrynType::Bool),
                    (VrynType::Float, VrynType::Int) => Ok(VrynType::Bool),
                    (VrynType::Str, VrynType::Str) => Ok(VrynType::Bool),
                    (VrynType::Bool, VrynType::Bool) => Ok(VrynType::Bool),
                    (VrynType::Any, _) | (_, VrynType::Any) => Ok(VrynType::Bool),
                    (VrynType::Error, _) | (_, VrynType::Error) => Ok(VrynType::Error),
                    _ => Err(TypeError::new(format!(
                        "Invalid operands for {:?}: {} and {}",
                        op, left_type, right_type
                    ))),
                }
            }

            BinaryOperator::And | BinaryOperator::Or => {
                match (left_type, right_type) {
                    (VrynType::Bool, VrynType::Bool) => Ok(VrynType::Bool),
                    (VrynType::Any, _) | (_, VrynType::Any) => Ok(VrynType::Bool),
                    (VrynType::Error, _) | (_, VrynType::Error) => Ok(VrynType::Error),
                    _ => Err(TypeError::new(format!(
                        "Logical operations require bool operands, got {} and {}",
                        left_type, right_type
                    ))),
                }
            }
        }
    }

    fn check_unary_op(
        &mut self,
        op: &UnaryOperator,
        operand_type: &VrynType,
    ) -> Result<VrynType, TypeError> {
        match op {
            UnaryOperator::Neg => {
                match operand_type {
                    VrynType::Int => Ok(VrynType::Int),
                    VrynType::Float => Ok(VrynType::Float),
                    VrynType::Any | VrynType::Error => Ok(operand_type.clone()),
                    _ => Err(TypeError::new(format!(
                        "Negation requires numeric operand, got {}",
                        operand_type
                    ))),
                }
            }
            UnaryOperator::Not => {
                match operand_type {
                    VrynType::Bool => Ok(VrynType::Bool),
                    VrynType::Any | VrynType::Error => Ok(VrynType::Bool),
                    _ => Err(TypeError::new(format!(
                        "Logical not requires bool operand, got {}",
                        operand_type
                    ))),
                }
            }
        }
    }

    fn parse_type_name(&self, type_name: &str) -> VrynType {
        match type_name {
            "i32" | "int" => VrynType::Int,
            "f64" | "float" => VrynType::Float,
            "str" | "string" => VrynType::Str,
            "bool" => VrynType::Bool,
            "void" | "none" => VrynType::Void,
            name => {
                // Try to look up user-defined types
                if self.env.lookup_struct(name).is_some() {
                    VrynType::Struct {
                        name: name.to_string(),
                        fields: self.env.lookup_struct(name).unwrap_or_default(),
                    }
                } else if self.env.lookup_enum(name).is_some() {
                    VrynType::Enum {
                        name: name.to_string(),
                    }
                } else {
                    // Unknown type name - treat as Any to avoid cascading errors
                    VrynType::Any
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::lexer::Lexer;

    fn parse(code: &str) -> Program {
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().expect("Lexer failed");
        let mut parser = Parser::new(tokens);
        parser.parse().expect("Parser failed")
    }

    fn check_type(code: &str) -> Vec<TypeError> {
        let program = parse(code);
        let mut checker = TypeChecker::new();
        checker.check_program(&program)
    }

    #[test]
    fn test_integer_literal() {
        let code = "let x = 42";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Integer literal should type-check");
    }

    #[test]
    fn test_float_literal() {
        let code = "let x = 3.14";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Float literal should type-check");
    }

    #[test]
    fn test_string_literal() {
        let code = "let x = \"hello\"";
        let errors = check_type(code);
        assert!(errors.is_empty(), "String literal should type-check");
    }

    #[test]
    fn test_bool_literal() {
        let code = "let x = true";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Bool literal should type-check");
    }

    #[test]
    fn test_type_annotation() {
        let code = "let x: i32 = 42";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Type annotation should match");
    }

    #[test]
    fn test_type_mismatch_annotation() {
        let code = "let x: i32 = 3.14";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Type mismatch should be caught");
        assert!(errors[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_int_plus_int() {
        let code = "let x = 5 + 3";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Int + Int should work");
    }

    #[test]
    fn test_float_plus_float() {
        let code = "let x = 3.5 + 2.5";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Float + Float should work");
    }

    #[test]
    fn test_int_plus_float() {
        let code = "let x = 5 + 3.14";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Int + Float should work (promotes to Float)");
    }

    #[test]
    fn test_string_plus_string() {
        let code = "let x = \"hello\" + \"world\"";
        let errors = check_type(code);
        assert!(errors.is_empty(), "String + String should work");
    }

    #[test]
    fn test_incompatible_binary_op() {
        let code = "let x = \"hello\" + 5";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "String + Int should fail");
    }

    #[test]
    fn test_function_call_correct_args() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }\nlet x = add(5, 3)";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Function call with correct args should work");
    }

    #[test]
    fn test_function_call_wrong_arg_count() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }\nlet x = add(5)";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Wrong number of arguments should fail");
    }

    #[test]
    fn test_function_call_wrong_arg_type() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }\nlet x = add(5, \"wrong\")";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Wrong argument type should fail");
    }

    #[test]
    fn test_undefined_variable() {
        let code = "let x = y + 5";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Undefined variable should fail");
    }

    #[test]
    fn test_array_type_inference() {
        let code = "let x = [1, 2, 3]";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Array type inference should work");
    }

    #[test]
    fn test_array_element_type_mismatch() {
        let code = "let x = [1, 2, \"three\"]";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Array element type mismatch should fail");
    }

    #[test]
    fn test_if_condition_must_be_bool() {
        let code = "if 42 { }";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "If condition must be bool");
    }

    #[test]
    fn test_comparison_returns_bool() {
        let code = "let x = 5 > 3";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Comparison should return bool");
    }

    #[test]
    fn test_mutability_check() {
        let code = "let x = 5\nx = 10";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Cannot reassign immutable variable");
    }

    #[test]
    fn test_mutable_variable_assignment() {
        let code = "let mut x = 5\nx = 10";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Can reassign mutable variable");
    }

    #[test]
    fn test_struct_definition_and_init() {
        let code = "struct Point { x: i32, y: i32 }\nlet p = Point { x: 1, y: 2 }";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Struct definition and init should work");
    }

    #[test]
    fn test_struct_field_type_mismatch() {
        let code = "struct Point { x: i32, y: i32 }\nlet p = Point { x: \"wrong\", y: 2 }";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Struct field type mismatch should fail");
    }

    #[test]
    fn test_return_type_checking() {
        let code = "fn get_five() -> i32 { return \"wrong\" }";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Return type mismatch should fail");
    }

    #[test]
    fn test_break_outside_loop() {
        let code = "break";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Break outside loop should fail");
    }

    #[test]
    fn test_break_inside_loop() {
        let code = "while true { break }";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Break inside loop should work");
    }

    #[test]
    fn test_for_loop_with_array() {
        let code = "let arr = [1, 2, 3]\nfor x in arr { }";
        let errors = check_type(code);
        assert!(errors.is_empty(), "For loop with array should work");
    }

    #[test]
    fn test_negation_on_int() {
        let code = "let x = -5";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Negation on int should work");
    }

    #[test]
    fn test_negation_on_bool() {
        let code = "let x = -true";
        let errors = check_type(code);
        assert!(!errors.is_empty(), "Negation on bool should fail");
    }

    #[test]
    fn test_logical_not_on_bool() {
        let code = "let x = !true";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Logical not on bool should work");
    }

    #[test]
    fn test_range_expression() {
        let code = "let r = 0..10";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Range expression should work");
    }

    #[test]
    fn test_array_indexing() {
        let code = "let arr = [1, 2, 3]\nlet x = arr[0]";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Array indexing should work");
    }

    #[test]
    fn test_builtin_len_function() {
        let code = "let arr = [1, 2, 3]\nlet x = len(arr)";
        let errors = check_type(code);
        assert!(errors.is_empty(), "Built-in len function should work");
    }
}
