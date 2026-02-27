//! LSP (Language Server Protocol) Foundation Module
//! Provides diagnostics, symbols, completions, and hover information for IDE support

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::{Program, Statement};
use std::collections::HashSet;

/// Diagnostic severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticSeverity::Error => write!(f, "error"),
            DiagnosticSeverity::Warning => write!(f, "warning"),
            DiagnosticSeverity::Info => write!(f, "info"),
        }
    }
}

/// A diagnostic message (error, warning, or info)
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub severity: DiagnosticSeverity,
}

impl Diagnostic {
    pub fn new(message: String, line: usize, column: usize, severity: DiagnosticSeverity) -> Self {
        Diagnostic { message, line, column, severity }
    }

    pub fn error(message: String, line: usize, column: usize) -> Self {
        Self::new(message, line, column, DiagnosticSeverity::Error)
    }

    pub fn warning(message: String, line: usize, column: usize) -> Self {
        Self::new(message, line, column, DiagnosticSeverity::Warning)
    }

    pub fn info(message: String, line: usize, column: usize) -> Self {
        Self::new(message, line, column, DiagnosticSeverity::Info)
    }
}

/// Symbol kind (function, variable, struct, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Constant,
    Struct,
    Enum,
    Trait,
    EnumVariant,
    Method,
    Parameter,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "function"),
            SymbolKind::Variable => write!(f, "variable"),
            SymbolKind::Constant => write!(f, "constant"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::EnumVariant => write!(f, "enum variant"),
            SymbolKind::Method => write!(f, "method"),
            SymbolKind::Parameter => write!(f, "parameter"),
        }
    }
}

/// A symbol (definition) in the code
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub column: usize,
    pub type_info: Option<String>, // e.g., "i64", "String", etc.
}

impl Symbol {
    pub fn new(name: String, kind: SymbolKind, line: usize, column: usize) -> Self {
        Symbol { name, kind, line, column, type_info: None }
    }

    pub fn with_type(mut self, type_info: String) -> Self {
        self.type_info = Some(type_info);
        self
    }
}

/// Result of analyzing source code
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: Vec<Symbol>,
    pub success: bool,
}

impl AnalysisResult {
    pub fn new(diagnostics: Vec<Diagnostic>, symbols: Vec<Symbol>, success: bool) -> Self {
        AnalysisResult { diagnostics, symbols, success }
    }
}

/// The Vryn code analyzer — provides IDE support
pub struct VrynAnalyzer;

impl VrynAnalyzer {
    /// Run full analysis pipeline on source code
    pub fn analyze(source: &str) -> AnalysisResult {
        let diagnostics = Self::get_diagnostics(source);
        let has_errors = diagnostics.iter().any(|d| d.severity == DiagnosticSeverity::Error);
        
        let symbols = if !has_errors {
            // Only try to extract symbols if parsing was successful
            match Self::parse(source) {
                Ok(program) => Self::get_symbols(&program),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        AnalysisResult::new(diagnostics, symbols, !has_errors)
    }

    /// Get all diagnostics (lex/parse errors)
    pub fn get_diagnostics(source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Try to lex
        let mut lexer = Lexer::new(source);
        match lexer.tokenize() {
            Ok(tokens) => {
                // Try to parse
                let mut parser = Parser::new(tokens);
                match parser.parse() {
                    Ok(_) => {
                        // No errors
                    }
                    Err(err) => {
                        // Parse error - extract line/column info if available
                        diagnostics.push(Diagnostic::error(err, 1, 1));
                    }
                }
            }
            Err(err) => {
                // Lex error - extract line/column info if available
                // Try to parse the error message for line:column info
                if let Some(pos) = err.find("at line") {
                    let rest = &err[pos + 7..];
                    if let Some(end_pos) = rest.find(',') {
                        if let Ok(line) = rest[..end_pos].trim().parse::<usize>() {
                            diagnostics.push(Diagnostic::error(err, line, 1));
                        } else {
                            diagnostics.push(Diagnostic::error(err, 1, 1));
                        }
                    } else {
                        diagnostics.push(Diagnostic::error(err, 1, 1));
                    }
                } else {
                    diagnostics.push(Diagnostic::error(err, 1, 1));
                }
            }
        }

        diagnostics
    }

    /// Extract all symbols from the AST
    pub fn get_symbols(program: &Program) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let mut line_counter = 1;

        for stmt in &program.statements {
            Self::extract_symbols_from_statement(stmt, &mut symbols, &mut line_counter);
        }

        symbols
    }

    /// Extract symbols from a statement
    fn extract_symbols_from_statement(stmt: &Statement, symbols: &mut Vec<Symbol>, line: &mut usize) {
        match stmt {
            Statement::Function { name, params, return_type, .. } => {
                let mut symbol = Symbol::new(name.clone(), SymbolKind::Function, *line, 1);
                if let Some(ret_type) = return_type {
                    symbol = symbol.with_type(ret_type.clone());
                }
                symbols.push(symbol);

                // Extract parameter symbols
                for param in params {
                    symbols.push(Symbol::new(
                        param.name.clone(),
                        SymbolKind::Parameter,
                        *line,
                        1,
                    ).with_type(param.type_name.clone()));
                }

                *line += 1;
            }
            Statement::Struct { name, fields, .. } => {
                let mut symbol = Symbol::new(name.clone(), SymbolKind::Struct, *line, 1);
                if !fields.is_empty() {
                    // Use first field type as type info
                    symbol = symbol.with_type(format!("struct with {} fields", fields.len()));
                }
                symbols.push(symbol);
                *line += 1;
            }
            Statement::Enum { name, variants, .. } => {
                let mut symbol = Symbol::new(name.clone(), SymbolKind::Enum, *line, 1);
                if !variants.is_empty() {
                    symbol = symbol.with_type(format!("enum with {} variants", variants.len()));
                }
                symbols.push(symbol);

                // Extract enum variant symbols
                for variant in variants {
                    symbols.push(Symbol::new(
                        variant.name.clone(),
                        SymbolKind::EnumVariant,
                        *line,
                        1,
                    ));
                }
                *line += 1;
            }
            Statement::Trait { name, methods, .. } => {
                let mut symbol = Symbol::new(name.clone(), SymbolKind::Trait, *line, 1);
                if !methods.is_empty() {
                    symbol = symbol.with_type(format!("trait with {} methods", methods.len()));
                }
                symbols.push(symbol);
                *line += 1;
            }
            Statement::Impl { trait_name, type_name, methods, .. } => {
                let impl_desc = if let Some(trait_n) = trait_name {
                    format!("impl {} for {}", trait_n, type_name)
                } else {
                    format!("impl {}", type_name)
                };

                for method in methods {
                    symbols.push(Symbol::new(
                        method.name.clone(),
                        SymbolKind::Method,
                        *line,
                        1,
                    ).with_type(impl_desc.clone()));
                }
                *line += 1;
            }
            Statement::Let { name, type_ann, .. } => {
                let mut symbol = Symbol::new(name.clone(), SymbolKind::Variable, *line, 1);
                if let Some(type_name) = type_ann {
                    symbol = symbol.with_type(type_name.clone());
                }
                symbols.push(symbol);
                *line += 1;
            }
            Statement::Const { name, .. } => {
                symbols.push(Symbol::new(name.clone(), SymbolKind::Constant, *line, 1));
                *line += 1;
            }
            Statement::Return(_) => {
                *line += 1;
            }
            Statement::If { then_body, else_body, .. } => {
                for s in then_body {
                    Self::extract_symbols_from_statement(s, symbols, line);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        Self::extract_symbols_from_statement(s, symbols, line);
                    }
                }
            }
            Statement::IfLet { then_body, else_body, .. } => {
                for s in then_body {
                    Self::extract_symbols_from_statement(s, symbols, line);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        Self::extract_symbols_from_statement(s, symbols, line);
                    }
                }
            }
            Statement::While { body, .. } => {
                for s in body {
                    Self::extract_symbols_from_statement(s, symbols, line);
                }
            }
            Statement::WhileLet { body, .. } => {
                for s in body {
                    Self::extract_symbols_from_statement(s, symbols, line);
                }
            }
            Statement::For { var, body, .. } => {
                symbols.push(Symbol::new(var.clone(), SymbolKind::Variable, *line, 1));
                for s in body {
                    Self::extract_symbols_from_statement(s, symbols, line);
                }
            }
            Statement::Expression(_) | Statement::Import { .. } | Statement::Break | Statement::Continue => {
                *line += 1;
            }
        }
    }

    /// Get completion suggestions
    pub fn get_completions(source: &str, _line: usize) -> Vec<String> {
        let mut completions = HashSet::new();

        // Add keywords
        let keywords = vec![
            "fn", "fun", "let", "var", "const", "if", "else", "while", "for", "in",
            "return", "break", "continue", "struct", "enum", "trait", "impl", "import",
            "use", "match", "async", "await", "spawn", "try", "catch", "true", "false",
            "self", "Self",
        ];
        for kw in keywords {
            completions.insert(kw.to_string());
        }

        // Add symbols from current source
        if let Ok(program) = Self::parse(source) {
            for symbol in Self::get_symbols(&program) {
                completions.insert(symbol.name);
            }
        }

        // Add built-in types
        let builtins = vec![
            "i32", "i64", "u32", "u64", "f32", "f64", "bool", "String", "Array",
            "Vec", "Map", "Option", "Result", "Unit",
        ];
        for builtin in builtins {
            completions.insert(builtin.to_string());
        }

        let mut result: Vec<String> = completions.into_iter().collect();
        result.sort();
        result
    }

    /// Find the definition of a symbol
    pub fn find_definition(source: &str, name: &str) -> Option<Symbol> {
        match Self::parse(source) {
            Ok(program) => {
                let symbols = Self::get_symbols(&program);
                symbols.into_iter().find(|s| s.name == name)
            }
            Err(_) => None,
        }
    }

    /// Get hover information for a symbol
    pub fn get_hover(source: &str, name: &str) -> Option<String> {
        match Self::parse(source) {
            Ok(program) => {
                let symbols = Self::get_symbols(&program);
                if let Some(symbol) = symbols.iter().find(|s| s.name == name) {
                    let mut hover = format!("({}) {}", symbol.kind, symbol.name);
                    if let Some(type_info) = &symbol.type_info {
                        hover.push_str(&format!(": {}", type_info));
                    }
                    return Some(hover);
                }
                None
            }
            Err(_) => None,
        }
    }

    /// Parse source code into a Program (helper)
    fn parse(source: &str) -> Result<Program, String> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_severity_display() {
        assert_eq!(DiagnosticSeverity::Error.to_string(), "error");
        assert_eq!(DiagnosticSeverity::Warning.to_string(), "warning");
        assert_eq!(DiagnosticSeverity::Info.to_string(), "info");
    }

    #[test]
    fn test_symbol_kind_display() {
        assert_eq!(SymbolKind::Function.to_string(), "function");
        assert_eq!(SymbolKind::Struct.to_string(), "struct");
        assert_eq!(SymbolKind::Variable.to_string(), "variable");
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error("test error".to_string(), 5, 10);
        assert_eq!(diag.message, "test error");
        assert_eq!(diag.line, 5);
        assert_eq!(diag.column, 10);
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn test_symbol_creation() {
        let symbol = Symbol::new("test_func".to_string(), SymbolKind::Function, 1, 1);
        assert_eq!(symbol.name, "test_func");
        assert_eq!(symbol.kind, SymbolKind::Function);
        assert_eq!(symbol.line, 1);
    }

    #[test]
    fn test_analyze_valid_code() {
        let source = "let x = 5";
        let result = VrynAnalyzer::analyze(source);
        assert!(result.success);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn test_analyze_invalid_code() {
        let source = "let x = ";
        let result = VrynAnalyzer::analyze(source);
        assert!(!result.success);
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn test_get_symbols_function() {
        let source = "fun add(a: i64, b: i64) -> i64 { a + b }";
        match VrynAnalyzer::parse(source) {
            Ok(program) => {
                let symbols = VrynAnalyzer::get_symbols(&program);
                assert!(!symbols.is_empty());
                let func = symbols.iter().find(|s| s.name == "add");
                assert!(func.is_some());
                assert_eq!(func.unwrap().kind, SymbolKind::Function);
            }
            Err(_) => panic!("Failed to parse"),
        }
    }

    #[test]
    fn test_get_symbols_struct() {
        let source = "struct Point { x: i64 y: i64 }";
        match VrynAnalyzer::parse(source) {
            Ok(program) => {
                let symbols = VrynAnalyzer::get_symbols(&program);
                let struct_sym = symbols.iter().find(|s| s.name == "Point");
                assert!(struct_sym.is_some());
                assert_eq!(struct_sym.unwrap().kind, SymbolKind::Struct);
            }
            Err(_) => panic!("Failed to parse"),
        }
    }

    #[test]
    fn test_get_symbols_enum() {
        let source = "enum Color { Red Green Blue }";
        match VrynAnalyzer::parse(source) {
            Ok(program) => {
                let symbols = VrynAnalyzer::get_symbols(&program);
                let enum_sym = symbols.iter().find(|s| s.name == "Color");
                assert!(enum_sym.is_some());
                assert_eq!(enum_sym.unwrap().kind, SymbolKind::Enum);
            }
            Err(_) => panic!("Failed to parse"),
        }
    }

    #[test]
    fn test_get_completions() {
        let source = "let x = 5";
        let completions = VrynAnalyzer::get_completions(source, 1);
        assert!(!completions.is_empty());
        assert!(completions.contains(&"fn".to_string()));
        assert!(completions.contains(&"let".to_string()));
        assert!(completions.contains(&"i64".to_string()));
    }

    #[test]
    fn test_find_definition() {
        let source = "let my_var = 42";
        let def = VrynAnalyzer::find_definition(source, "my_var");
        assert!(def.is_some());
        assert_eq!(def.unwrap().name, "my_var");
    }

    #[test]
    fn test_get_hover() {
        let source = "fun greet() { }";
        let hover = VrynAnalyzer::get_hover(source, "greet");
        assert!(hover.is_some());
        let hover_text = hover.unwrap();
        assert!(hover_text.contains("greet"));
        assert!(hover_text.contains("function"));
    }
}
