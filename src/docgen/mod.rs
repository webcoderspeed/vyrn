//! Documentation Generator for Vryn
//! Extracts documentation from source code and generates HTML/Markdown docs

use crate::parser::ast::*;
use std::collections::HashMap;

/// Kind of documented item
#[derive(Debug, Clone, PartialEq)]
pub enum DocItemKind {
    Function,
    Struct,
    Enum,
    Trait,
    Method,
}

impl DocItemKind {
    #[allow(dead_code)]
    fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "Function",
            Self::Struct => "Struct",
            Self::Enum => "Enum",
            Self::Trait => "Trait",
            Self::Method => "Method",
        }
    }
}

/// A documented parameter
#[derive(Debug, Clone)]
pub struct DocParam {
    pub name: String,
    pub type_name: String,
}

/// A documented item (function, struct, enum, trait, method)
#[derive(Debug, Clone)]
pub struct DocItem {
    pub name: String,
    pub kind: DocItemKind,
    pub description: Option<String>,
    pub params: Vec<DocParam>,
    pub return_type: Option<String>,
    pub fields: Vec<(String, String)>, // (field_name, field_type) for structs
    pub variants: Vec<String>,          // variant names for enums
    pub methods: Vec<String>,           // method names for traits
    pub impl_type: Option<String>,      // type name for impl blocks
}

/// Generator that extracts documentation from a Vryn program
pub struct DocGenerator;

impl DocGenerator {
    /// Extract all documentation items from a program
    pub fn extract_docs(program: &Program) -> Vec<DocItem> {
        let mut docs = Vec::new();

        for stmt in &program.statements {
            match stmt {
                Statement::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let doc_item = DocItem {
                        name: name.clone(),
                        kind: DocItemKind::Function,
                        description: None,
                        params: params
                            .iter()
                            .map(|p| DocParam {
                                name: p.name.clone(),
                                type_name: p.type_name.clone(),
                            })
                            .collect(),
                        return_type: return_type.clone(),
                        fields: Vec::new(),
                        variants: Vec::new(),
                        methods: Vec::new(),
                        impl_type: None,
                    };
                    docs.push(doc_item);
                }
                Statement::Struct { name, fields } => {
                    let doc_item = DocItem {
                        name: name.clone(),
                        kind: DocItemKind::Struct,
                        description: None,
                        params: Vec::new(),
                        return_type: None,
                        fields: fields
                            .iter()
                            .map(|f| (f.name.clone(), f.type_name.clone()))
                            .collect(),
                        variants: Vec::new(),
                        methods: Vec::new(),
                        impl_type: None,
                    };
                    docs.push(doc_item);
                }
                Statement::Enum { name, variants } => {
                    let variant_names = variants.iter().map(|v| v.name.clone()).collect();
                    let doc_item = DocItem {
                        name: name.clone(),
                        kind: DocItemKind::Enum,
                        description: None,
                        params: Vec::new(),
                        return_type: None,
                        fields: Vec::new(),
                        variants: variant_names,
                        methods: Vec::new(),
                        impl_type: None,
                    };
                    docs.push(doc_item);
                }
                Statement::Trait { name, methods } => {
                    let method_names = methods.iter().map(|m| m.name.clone()).collect();
                    let doc_item = DocItem {
                        name: name.clone(),
                        kind: DocItemKind::Trait,
                        description: None,
                        params: Vec::new(),
                        return_type: None,
                        fields: Vec::new(),
                        variants: Vec::new(),
                        methods: method_names,
                        impl_type: None,
                    };
                    docs.push(doc_item);
                }
                Statement::Impl {
                    trait_name: _,
                    type_name,
                    methods,
                } => {
                    for method in methods {
                        let doc_item = DocItem {
                            name: method.name.clone(),
                            kind: DocItemKind::Method,
                            description: None,
                            params: method
                                .params
                                .iter()
                                .map(|p| DocParam {
                                    name: p.name.clone(),
                                    type_name: p.type_name.clone(),
                                })
                                .collect(),
                            return_type: method.return_type.clone(),
                            fields: Vec::new(),
                            variants: Vec::new(),
                            methods: Vec::new(),
                            impl_type: Some(type_name.clone()),
                        };
                        docs.push(doc_item);
                    }
                }
                _ => {}
            }
        }

        docs
    }

    /// Generate Markdown documentation from doc items
    pub fn generate_markdown(docs: &[DocItem]) -> String {
        let mut output = String::new();
        output.push_str("# Vryn Documentation\n\n");

        // Categorize items
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut traits = Vec::new();
        let mut methods_by_type: HashMap<String, Vec<&DocItem>> = HashMap::new();

        for doc in docs {
            match doc.kind {
                DocItemKind::Function => functions.push(doc),
                DocItemKind::Struct => structs.push(doc),
                DocItemKind::Enum => enums.push(doc),
                DocItemKind::Trait => traits.push(doc),
                DocItemKind::Method => {
                    if let Some(impl_type) = &doc.impl_type {
                        methods_by_type
                            .entry(impl_type.clone())
                            .or_insert_with(Vec::new)
                            .push(doc);
                    }
                }
            }
        }

        // Functions section
        if !functions.is_empty() {
            output.push_str("## Functions\n\n");
            for func in &functions {
                output.push_str(&format!("### `{}`\n\n", func.name));

                if let Some(desc) = &func.description {
                    output.push_str(&format!("{}\n\n", desc));
                }

                if !func.params.is_empty() {
                    output.push_str("**Parameters:**\n\n");
                    for param in &func.params {
                        output.push_str(&format!("- `{}`: {}\n", param.name, param.type_name));
                    }
                    output.push('\n');
                }

                if let Some(ret_type) = &func.return_type {
                    output.push_str(&format!("**Returns:** `{}`\n\n", ret_type));
                }
            }
        }

        // Structs section
        if !structs.is_empty() {
            output.push_str("## Structs\n\n");
            for s in &structs {
                output.push_str(&format!("### `{}`\n\n", s.name));

                if let Some(desc) = &s.description {
                    output.push_str(&format!("{}\n\n", desc));
                }

                if !s.fields.is_empty() {
                    output.push_str("**Fields:**\n\n");
                    for (field_name, field_type) in &s.fields {
                        output.push_str(&format!("- `{}`: {}\n", field_name, field_type));
                    }
                    output.push('\n');
                }
            }
        }

        // Enums section
        if !enums.is_empty() {
            output.push_str("## Enums\n\n");
            for e in &enums {
                output.push_str(&format!("### `{}`\n\n", e.name));

                if let Some(desc) = &e.description {
                    output.push_str(&format!("{}\n\n", desc));
                }

                if !e.variants.is_empty() {
                    output.push_str("**Variants:**\n\n");
                    for variant in &e.variants {
                        output.push_str(&format!("- `{}`\n", variant));
                    }
                    output.push('\n');
                }
            }
        }

        // Traits section
        if !traits.is_empty() {
            output.push_str("## Traits\n\n");
            for t in &traits {
                output.push_str(&format!("### `{}`\n\n", t.name));

                if let Some(desc) = &t.description {
                    output.push_str(&format!("{}\n\n", desc));
                }

                if !t.methods.is_empty() {
                    output.push_str("**Methods:**\n\n");
                    for method in &t.methods {
                        output.push_str(&format!("- `{}`\n", method));
                    }
                    output.push('\n');
                }
            }
        }

        // Methods by type
        if !methods_by_type.is_empty() {
            output.push_str("## Implementations\n\n");
            let mut types: Vec<_> = methods_by_type.keys().collect();
            types.sort();

            for type_name in types {
                output.push_str(&format!("### Methods for `{}`\n\n", type_name));

                if let Some(methods) = methods_by_type.get(type_name) {
                    for method in methods {
                        output.push_str(&format!("#### `{}`\n\n", method.name));

                        if let Some(desc) = &method.description {
                            output.push_str(&format!("{}\n\n", desc));
                        }

                        if !method.params.is_empty() {
                            output.push_str("**Parameters:**\n\n");
                            for param in &method.params {
                                output.push_str(&format!("- `{}`: {}\n", param.name, param.type_name));
                            }
                            output.push('\n');
                        }

                        if let Some(ret_type) = &method.return_type {
                            output.push_str(&format!("**Returns:** `{}`\n\n", ret_type));
                        }
                    }
                }
            }
        }

        output
    }

    /// Generate HTML documentation from doc items
    pub fn generate_html(docs: &[DocItem]) -> String {
        let mut output = String::new();

        // HTML header
        output.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vryn Documentation</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            line-height: 1.6;
            color: #333;
            background: #f5f5f5;
        }
        
        .container {
            display: flex;
            min-height: 100vh;
        }
        
        .sidebar {
            width: 280px;
            background: #2c3e50;
            color: white;
            padding: 20px;
            overflow-y: auto;
            position: sticky;
            top: 0;
            height: 100vh;
        }
        
        .sidebar h1 {
            font-size: 24px;
            margin-bottom: 30px;
            padding-bottom: 15px;
            border-bottom: 2px solid #34495e;
        }
        
        .sidebar-section {
            margin-bottom: 25px;
        }
        
        .sidebar-section-title {
            font-size: 12px;
            text-transform: uppercase;
            color: #95a5a6;
            margin-bottom: 10px;
            font-weight: bold;
            letter-spacing: 1px;
        }
        
        .sidebar-link {
            display: block;
            padding: 8px 12px;
            margin-bottom: 5px;
            color: #ecf0f1;
            text-decoration: none;
            border-radius: 4px;
            font-size: 14px;
            transition: background-color 0.2s;
        }
        
        .sidebar-link:hover {
            background-color: #34495e;
        }
        
        .sidebar-link.function::before { content: "ƒ "; color: #3498db; }
        .sidebar-link.struct::before { content: "◻ "; color: #e74c3c; }
        .sidebar-link.enum::before { content: "⬚ "; color: #f39c12; }
        .sidebar-link.trait::before { content: "⌘ "; color: #9b59b6; }
        .sidebar-link.method::before { content: "• "; color: #1abc9c; }
        
        .main {
            flex: 1;
            padding: 40px;
            max-width: 900px;
        }
        
        .main h1 {
            color: #2c3e50;
            margin-bottom: 30px;
            font-size: 32px;
            border-bottom: 3px solid #3498db;
            padding-bottom: 10px;
        }
        
        .main h2 {
            color: #2c3e50;
            margin-top: 40px;
            margin-bottom: 20px;
            font-size: 24px;
            border-left: 4px solid #3498db;
            padding-left: 12px;
        }
        
        .main h3 {
            color: #34495e;
            margin-top: 25px;
            margin-bottom: 15px;
            font-size: 18px;
            font-family: 'Courier New', monospace;
        }
        
        .doc-item {
            background: white;
            padding: 20px;
            margin-bottom: 20px;
            border-radius: 6px;
            border-left: 4px solid #3498db;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        
        .item-name {
            font-family: 'Courier New', monospace;
            font-size: 18px;
            font-weight: bold;
            color: #2c3e50;
            margin-bottom: 10px;
        }
        
        .item-kind {
            display: inline-block;
            padding: 4px 8px;
            font-size: 12px;
            font-weight: bold;
            text-transform: uppercase;
            border-radius: 3px;
            margin-bottom: 10px;
            margin-right: 10px;
        }
        
        .kind-function { background: #d4e6f1; color: #1f618d; }
        .kind-struct { background: #fadbd8; color: #922b21; }
        .kind-enum { background: #fdebd0; color: #b7590d; }
        .kind-trait { background: #ebdef0; color: #512e5f; }
        .kind-method { background: #d1f2eb; color: #0e6251; }
        
        table {
            width: 100%;
            border-collapse: collapse;
            margin: 15px 0;
            font-size: 14px;
        }
        
        table th {
            background: #ecf0f1;
            padding: 12px;
            text-align: left;
            font-weight: bold;
            color: #2c3e50;
            border-bottom: 2px solid #bdc3c7;
        }
        
        table td {
            padding: 10px 12px;
            border-bottom: 1px solid #ecf0f1;
        }
        
        table tr:hover {
            background: #f8f9fa;
        }
        
        code {
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
            color: #c7254e;
        }
        
        .params, .fields, .variants, .methods, .returns {
            margin: 15px 0;
        }
        
        .params-label, .fields-label, .variants-label, .methods-label, .returns-label {
            font-weight: bold;
            color: #2c3e50;
            margin-bottom: 10px;
            font-size: 14px;
        }
        
        .param-item, .field-item, .variant-item, .method-item {
            padding: 8px 12px;
            margin-bottom: 5px;
            background: #f9f9f9;
            border-left: 3px solid #3498db;
            border-radius: 3px;
        }
        
        @media (max-width: 768px) {
            .container {
                flex-direction: column;
            }
            
            .sidebar {
                width: 100%;
                height: auto;
                position: relative;
            }
            
            .main {
                padding: 20px;
            }
        }
    </style>
</head>
<body>
"#);

        // Categorize items
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut traits = Vec::new();
        let mut methods_by_type: HashMap<String, Vec<&DocItem>> = HashMap::new();

        for doc in docs {
            match doc.kind {
                DocItemKind::Function => functions.push(doc),
                DocItemKind::Struct => structs.push(doc),
                DocItemKind::Enum => enums.push(doc),
                DocItemKind::Trait => traits.push(doc),
                DocItemKind::Method => {
                    if let Some(impl_type) = &doc.impl_type {
                        methods_by_type
                            .entry(impl_type.clone())
                            .or_insert_with(Vec::new)
                            .push(doc);
                    }
                }
            }
        }

        // Sidebar
        output.push_str(r#"    <div class="container">
        <div class="sidebar">
            <h1>Vryn Docs</h1>
"#);

        if !functions.is_empty() {
            output.push_str("            <div class=\"sidebar-section\">\n");
            output.push_str("                <div class=\"sidebar-section-title\">Functions</div>\n");
            for func in &functions {
                output.push_str(&format!(
                    "                <a class=\"sidebar-link function\" href=\"#fn-{}\">{}</a>\n",
                    func.name, func.name
                ));
            }
            output.push_str("            </div>\n");
        }

        if !structs.is_empty() {
            output.push_str("            <div class=\"sidebar-section\">\n");
            output.push_str("                <div class=\"sidebar-section-title\">Structs</div>\n");
            for s in &structs {
                output.push_str(&format!(
                    "                <a class=\"sidebar-link struct\" href=\"#struct-{}\">{}</a>\n",
                    s.name, s.name
                ));
            }
            output.push_str("            </div>\n");
        }

        if !enums.is_empty() {
            output.push_str("            <div class=\"sidebar-section\">\n");
            output.push_str("                <div class=\"sidebar-section-title\">Enums</div>\n");
            for e in &enums {
                output.push_str(&format!(
                    "                <a class=\"sidebar-link enum\" href=\"#enum-{}\">{}</a>\n",
                    e.name, e.name
                ));
            }
            output.push_str("            </div>\n");
        }

        if !traits.is_empty() {
            output.push_str("            <div class=\"sidebar-section\">\n");
            output.push_str("                <div class=\"sidebar-section-title\">Traits</div>\n");
            for t in &traits {
                output.push_str(&format!(
                    "                <a class=\"sidebar-link trait\" href=\"#trait-{}\">{}</a>\n",
                    t.name, t.name
                ));
            }
            output.push_str("            </div>\n");
        }

        if !methods_by_type.is_empty() {
            output.push_str("            <div class=\"sidebar-section\">\n");
            output.push_str("                <div class=\"sidebar-section-title\">Implementations</div>\n");
            let mut types: Vec<_> = methods_by_type.keys().collect();
            types.sort();
            for type_name in types {
                output.push_str(&format!(
                    "                <a class=\"sidebar-link method\" href=\"#impl-{}\">{}</a>\n",
                    type_name, type_name
                ));
            }
            output.push_str("            </div>\n");
        }

        output.push_str("        </div>\n");

        // Main content
        output.push_str("        <div class=\"main\">\n");
        output.push_str("            <h1>Vryn Documentation</h1>\n");

        // Functions section
        if !functions.is_empty() {
            output.push_str("            <h2>Functions</h2>\n");
            for func in &functions {
                output.push_str(&format!(
                    "            <div class=\"doc-item\" id=\"fn-{}\">\n",
                    func.name
                ));
                output.push_str(&format!(
                    "                <div class=\"item-kind kind-function\">Function</div>\n"
                ));
                output.push_str(&format!("                <div class=\"item-name\">{}</div>\n", func.name));

                if let Some(desc) = &func.description {
                    output.push_str(&format!("                <p>{}</p>\n", desc));
                }

                if !func.params.is_empty() {
                    output.push_str("                <div class=\"params\">\n");
                    output.push_str("                    <div class=\"params-label\">Parameters:</div>\n");
                    output.push_str("                    <table>\n");
                    output.push_str(
                        "                        <thead><tr><th>Name</th><th>Type</th></tr></thead>\n",
                    );
                    output.push_str("                        <tbody>\n");
                    for param in &func.params {
                        output.push_str(&format!(
                            "                            <tr><td><code>{}</code></td><td><code>{}</code></td></tr>\n",
                            param.name, param.type_name
                        ));
                    }
                    output.push_str("                        </tbody>\n");
                    output.push_str("                    </table>\n");
                    output.push_str("                </div>\n");
                }

                if let Some(ret_type) = &func.return_type {
                    output.push_str("                <div class=\"returns\">\n");
                    output.push_str(&format!(
                        "                    <div class=\"returns-label\">Returns: <code>{}</code></div>\n",
                        ret_type
                    ));
                    output.push_str("                </div>\n");
                }

                output.push_str("            </div>\n");
            }
        }

        // Structs section
        if !structs.is_empty() {
            output.push_str("            <h2>Structs</h2>\n");
            for s in &structs {
                output.push_str(&format!(
                    "            <div class=\"doc-item\" id=\"struct-{}\">\n",
                    s.name
                ));
                output.push_str(&format!(
                    "                <div class=\"item-kind kind-struct\">Struct</div>\n"
                ));
                output.push_str(&format!("                <div class=\"item-name\">{}</div>\n", s.name));

                if let Some(desc) = &s.description {
                    output.push_str(&format!("                <p>{}</p>\n", desc));
                }

                if !s.fields.is_empty() {
                    output.push_str("                <div class=\"fields\">\n");
                    output.push_str("                    <div class=\"fields-label\">Fields:</div>\n");
                    output.push_str("                    <table>\n");
                    output.push_str(
                        "                        <thead><tr><th>Name</th><th>Type</th></tr></thead>\n",
                    );
                    output.push_str("                        <tbody>\n");
                    for (field_name, field_type) in &s.fields {
                        output.push_str(&format!(
                            "                            <tr><td><code>{}</code></td><td><code>{}</code></td></tr>\n",
                            field_name, field_type
                        ));
                    }
                    output.push_str("                        </tbody>\n");
                    output.push_str("                    </table>\n");
                    output.push_str("                </div>\n");
                }

                output.push_str("            </div>\n");
            }
        }

        // Enums section
        if !enums.is_empty() {
            output.push_str("            <h2>Enums</h2>\n");
            for e in &enums {
                output.push_str(&format!(
                    "            <div class=\"doc-item\" id=\"enum-{}\">\n",
                    e.name
                ));
                output.push_str(&format!(
                    "                <div class=\"item-kind kind-enum\">Enum</div>\n"
                ));
                output.push_str(&format!("                <div class=\"item-name\">{}</div>\n", e.name));

                if let Some(desc) = &e.description {
                    output.push_str(&format!("                <p>{}</p>\n", desc));
                }

                if !e.variants.is_empty() {
                    output.push_str("                <div class=\"variants\">\n");
                    output.push_str("                    <div class=\"variants-label\">Variants:</div>\n");
                    for variant in &e.variants {
                        output.push_str(&format!(
                            "                    <div class=\"variant-item\"><code>{}</code></div>\n",
                            variant
                        ));
                    }
                    output.push_str("                </div>\n");
                }

                output.push_str("            </div>\n");
            }
        }

        // Traits section
        if !traits.is_empty() {
            output.push_str("            <h2>Traits</h2>\n");
            for t in &traits {
                output.push_str(&format!(
                    "            <div class=\"doc-item\" id=\"trait-{}\">\n",
                    t.name
                ));
                output.push_str(&format!(
                    "                <div class=\"item-kind kind-trait\">Trait</div>\n"
                ));
                output.push_str(&format!("                <div class=\"item-name\">{}</div>\n", t.name));

                if let Some(desc) = &t.description {
                    output.push_str(&format!("                <p>{}</p>\n", desc));
                }

                if !t.methods.is_empty() {
                    output.push_str("                <div class=\"methods\">\n");
                    output.push_str("                    <div class=\"methods-label\">Methods:</div>\n");
                    for method in &t.methods {
                        output.push_str(&format!(
                            "                    <div class=\"method-item\"><code>{}</code></div>\n",
                            method
                        ));
                    }
                    output.push_str("                </div>\n");
                }

                output.push_str("            </div>\n");
            }
        }

        // Methods by type
        if !methods_by_type.is_empty() {
            output.push_str("            <h2>Implementations</h2>\n");
            let mut types: Vec<_> = methods_by_type.keys().collect();
            types.sort();

            for type_name in types {
                output.push_str(&format!(
                    "            <div class=\"doc-item\" id=\"impl-{}\">\n",
                    type_name
                ));
                output.push_str(&format!(
                    "                <div class=\"item-kind kind-method\">Implementation</div>\n"
                ));
                output.push_str(&format!(
                    "                <div class=\"item-name\">impl {}</div>\n",
                    type_name
                ));

                if let Some(methods) = methods_by_type.get(type_name) {
                    output.push_str("                <div class=\"methods\">\n");
                    for method in methods {
                        output.push_str(&format!(
                            "                    <div class=\"method-item\"><code>{}</code></div>\n",
                            method.name
                        ));
                    }
                    output.push_str("                </div>\n");
                }

                output.push_str("            </div>\n");
            }
        }

        output.push_str("        </div>\n");
        output.push_str("    </div>\n");

        // HTML footer
        output.push_str(
            r#"</body>
</html>
"#,
        );

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_empty_program() {
        let program = Program {
            statements: vec![],
        };
        let docs = DocGenerator::extract_docs(&program);
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn test_extract_function() {
        let program = Program {
            statements: vec![Statement::Function {
                name: "test_func".to_string(),
                params: vec![
                    Param {
                        name: "x".to_string(),
                        type_name: "i32".to_string(),
                    },
                    Param {
                        name: "y".to_string(),
                        type_name: "i32".to_string(),
                    },
                ],
                return_type: Some("i32".to_string()),
                body: vec![],
                is_async: false,
            }],
        };

        let docs = DocGenerator::extract_docs(&program);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].name, "test_func");
        assert_eq!(docs[0].kind, DocItemKind::Function);
        assert_eq!(docs[0].params.len(), 2);
        assert_eq!(docs[0].params[0].name, "x");
        assert_eq!(docs[0].params[0].type_name, "i32");
        assert_eq!(docs[0].return_type, Some("i32".to_string()));
    }

    #[test]
    fn test_extract_struct() {
        let program = Program {
            statements: vec![Statement::Struct {
                name: "Point".to_string(),
                fields: vec![
                    Field {
                        name: "x".to_string(),
                        type_name: "f64".to_string(),
                    },
                    Field {
                        name: "y".to_string(),
                        type_name: "f64".to_string(),
                    },
                ],
            }],
        };

        let docs = DocGenerator::extract_docs(&program);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].name, "Point");
        assert_eq!(docs[0].kind, DocItemKind::Struct);
        assert_eq!(docs[0].fields.len(), 2);
        assert_eq!(docs[0].fields[0].0, "x");
        assert_eq!(docs[0].fields[0].1, "f64");
    }

    #[test]
    fn test_extract_enum() {
        let program = Program {
            statements: vec![Statement::Enum {
                name: "Result".to_string(),
                variants: vec![
                    EnumVariant {
                        name: "Ok".to_string(),
                        fields: vec![],
                    },
                    EnumVariant {
                        name: "Err".to_string(),
                        fields: vec![],
                    },
                ],
            }],
        };

        let docs = DocGenerator::extract_docs(&program);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].name, "Result");
        assert_eq!(docs[0].kind, DocItemKind::Enum);
        assert_eq!(docs[0].variants.len(), 2);
        assert!(docs[0].variants.contains(&"Ok".to_string()));
        assert!(docs[0].variants.contains(&"Err".to_string()));
    }

    #[test]
    fn test_extract_trait() {
        let program = Program {
            statements: vec![Statement::Trait {
                name: "Iterator".to_string(),
                methods: vec![TraitMethod {
                    name: "next".to_string(),
                    params: vec![],
                    return_type: Some("Option".to_string()),
                }],
            }],
        };

        let docs = DocGenerator::extract_docs(&program);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].name, "Iterator");
        assert_eq!(docs[0].kind, DocItemKind::Trait);
        assert_eq!(docs[0].methods.len(), 1);
        assert_eq!(docs[0].methods[0], "next");
    }

    #[test]
    fn test_generate_markdown() {
        let docs = vec![
            DocItem {
                name: "add".to_string(),
                kind: DocItemKind::Function,
                description: Some("Add two numbers".to_string()),
                params: vec![
                    DocParam {
                        name: "a".to_string(),
                        type_name: "i32".to_string(),
                    },
                    DocParam {
                        name: "b".to_string(),
                        type_name: "i32".to_string(),
                    },
                ],
                return_type: Some("i32".to_string()),
                fields: vec![],
                variants: vec![],
                methods: vec![],
                impl_type: None,
            },
        ];

        let markdown = DocGenerator::generate_markdown(&docs);
        assert!(markdown.contains("# Vryn Documentation"));
        assert!(markdown.contains("## Functions"));
        assert!(markdown.contains("### `add`"));
        assert!(markdown.contains("Add two numbers"));
        assert!(markdown.contains("`a`: i32"));
        assert!(markdown.contains("`b`: i32"));
        assert!(markdown.contains("**Returns:** `i32`"));
    }

    #[test]
    fn test_generate_html() {
        let docs = vec![
            DocItem {
                name: "multiply".to_string(),
                kind: DocItemKind::Function,
                description: Some("Multiply two numbers".to_string()),
                params: vec![
                    DocParam {
                        name: "x".to_string(),
                        type_name: "f64".to_string(),
                    },
                    DocParam {
                        name: "y".to_string(),
                        type_name: "f64".to_string(),
                    },
                ],
                return_type: Some("f64".to_string()),
                fields: vec![],
                variants: vec![],
                methods: vec![],
                impl_type: None,
            },
        ];

        let html = DocGenerator::generate_html(&docs);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Vryn Documentation"));
        assert!(html.contains("multiply"));
        assert!(html.contains("Multiply two numbers"));
        assert!(html.contains("x</code></td><td><code>f64"));
    }

    #[test]
    fn test_generate_html_with_struct() {
        let docs = vec![DocItem {
            name: "User".to_string(),
            kind: DocItemKind::Struct,
            description: Some("User data structure".to_string()),
            params: vec![],
            return_type: None,
            fields: vec![
                ("id".to_string(), "i32".to_string()),
                ("name".to_string(), "string".to_string()),
            ],
            variants: vec![],
            methods: vec![],
            impl_type: None,
        }];

        let html = DocGenerator::generate_html(&docs);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Structs"));
        assert!(html.contains("User"));
        assert!(html.contains("User data structure"));
        assert!(html.contains("id</code></td><td><code>i32"));
        assert!(html.contains("name</code></td><td><code>string"));
    }
}
