mod lexer;
mod parser;
mod codegen;
mod typechecker;
mod formatter;
mod package;
mod ccodegen;
mod docgen;
mod lsp;

use std::env;
use std::fs;
use std::io::{self, Write, BufRead};

use lexer::Lexer;
use parser::Parser;
use codegen::Interpreter;
use typechecker::TypeChecker;
use formatter::Formatter;
use lsp::VrynAnalyzer;

const VERSION: &str = "0.1.0-alpha";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("\x1b[31merror:\x1b[0m No file specified");
                eprintln!("Usage: vryn run <file.vn>");
                std::process::exit(1);
            }
            run_file(&args[2]);
        }
        "repl" => run_repl(),
        "version" | "--version" | "-v" => {
            println!("vryn {}", VERSION);
        }
        "help" | "--help" | "-h" => print_usage(),
        "check" => {
            if args.len() < 3 {
                eprintln!("\x1b[31merror:\x1b[0m No file specified");
                std::process::exit(1);
            }
            check_file(&args[2]);
        }
        "tokens" => {
            if args.len() < 3 {
                eprintln!("Usage: vryn tokens <file.vn>");
                std::process::exit(1);
            }
            show_tokens(&args[2]);
        }
        "ast" => {
            if args.len() < 3 {
                eprintln!("Usage: vryn ast <file.vn>");
                std::process::exit(1);
            }
            show_ast(&args[2]);
        }
        "analyze" => {
            if args.len() < 3 {
                eprintln!("Usage: vryn analyze <file.vn>");
                std::process::exit(1);
            }
            analyze_file(&args[2]);
        }
        "new" => {
            if args.len() < 3 {
                eprintln!("\x1b[31merror:\x1b[0m No project name specified");
                eprintln!("Usage: vryn new <name>");
                std::process::exit(1);
            }
            new_project(&args[2]);
        }
        "init" => {
            init_project();
        }
        "test" => {
            let file = if args.len() > 2 { Some(args[2].as_str()) } else { None };
            test_file(file);
        }
        "fmt" => {
            if args.len() < 3 {
                eprintln!("Usage: vryn format <file.vn>");
                std::process::exit(1);
            }
            format_file(&args[2]);
        }
        "compile" => {
            if args.len() < 3 {
                eprintln!("[31merror:[0m No file specified");
                eprintln!("Usage: vryn compile <file.vn>");
                std::process::exit(1);
            }
            compile_file(&args[2]);
        }
        "doc" => {
            if args.len() < 3 {
                eprintln!("Usage: vryn doc <file.vn>");
                std::process::exit(1);
            }
            generate_docs(&args[2]);
        }
        other => {
            if other.ends_with(".vn") {
                run_file(other);
            } else {
                eprintln!("\x1b[31merror:\x1b[0m Unknown command '{}'", other);
                print_usage();
                std::process::exit(1);
            }
        }
    }
}

fn print_usage() {
    println!(r#"
  Vryn Programming Language
  Fast | Safe | Easy | Universal

  USAGE:
    vryn <command> [options]

  COMMANDS:
    run <file.vn>     Compile and run a Vryn program
    compile <file.vn> Transpile Vryn to C code
    new <name>        Create a new Vryn project
    init              Initialize a Vryn project in current directory
    test [file.vn]    Run test functions (test_*)
    fmt <file.vn>     Format code with 4-space indentation
    doc <file.vn>     Generate HTML/Markdown documentation
    repl              Start interactive REPL
    check <file.vn>   Type-check without running
    version           Show version info
    help              Show this help message

  DEBUG:
    tokens <file.vn>   Show lexer output (tokens)
    ast <file.vn>      Show parser output (AST)
    analyze <file.vn>  Analyze code and show diagnostics/symbols

  EXAMPLES:
    vryn run hello.vn
    vryn repl
    vryn check main.vn

  VERSION: {}
"#, VERSION);
}

fn run_file(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m Cannot read file '{}': {}", path, e);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("\x1b[31merror[Lexer]:\x1b[0m {}", e);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("\x1b[31merror[Parser]:\x1b[0m {}", e);
            std::process::exit(1);
        }
    };

    let has_main = program.statements.iter().any(|s| {
        matches!(s, parser::ast::Statement::Function { name, .. } if name == "main")
    });

    let mut interpreter = Interpreter::new();

    if let Err(e) = interpreter.run(&program) {
        eprintln!("\x1b[31merror[Runtime]:\x1b[0m {}", e);
        std::process::exit(1);
    }

    if has_main {
        let call_main = parser::ast::Program {
            statements: vec![
                parser::ast::Statement::Expression(
                    parser::ast::Expression::Call {
                        function: Box::new(parser::ast::Expression::Identifier("main".to_string())),
                        args: vec![],
                    }
                )
            ],
        };
        if let Err(e) = interpreter.run(&call_main) {
            eprintln!("\x1b[31merror[Runtime]:\x1b[0m {}", e);
            std::process::exit(1);
        }
    }
}

fn run_repl() {
    println!("Vryn {} REPL", VERSION);
    println!("Type :help for help, :quit to exit\n");

    let stdin = io::stdin();
    let mut interpreter = Interpreter::new();

    loop {
        print!("\x1b[36mvryn>\x1b[0m ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() { continue; }

        match line {
            ":quit" | ":q" | ":exit" => {
                println!("Goodbye!");
                break;
            }
            ":help" | ":h" => {
                println!("  :quit    Exit REPL");
                println!("  :help    Show this help");
                println!("  :clear   Clear screen");
                continue;
            }
            ":clear" => {
                print!("\x1b[2J\x1b[H");
                continue;
            }
            _ => {}
        }

        let mut lexer = Lexer::new(line);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("\x1b[31merror:\x1b[0m {}", e);
                continue;
            }
        };

        let mut parser = Parser::new(tokens);
        let program = match parser.parse() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("\x1b[31merror:\x1b[0m {}", e);
                continue;
            }
        };

        if let Err(e) = interpreter.run(&program) {
            eprintln!("\x1b[31merror:\x1b[0m {}", e);
        }
    }
}

fn check_file(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m Cannot read file '{}': {}", path, e);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("\x1b[31merror[Lexer]:\x1b[0m {}", e);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("\x1b[31merror[Parser]:\x1b[0m {}", e);
            std::process::exit(1);
        }
    };

    let mut type_checker = TypeChecker::new();
    let type_errors = type_checker.check_program(&program);

    if !type_errors.is_empty() {
        for err in &type_errors {
            eprintln!("\x1b[31merror[TypeChecker]:\x1b[0m {}", err);
        }
        std::process::exit(1);
    } else {
        println!("\x1b[32m✓\x1b[0m {} — no errors found", path);
    }
}

fn show_tokens(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot read '{}': {}", path, e);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    match lexer.tokenize() {
        Ok(tokens) => {
            for tok in &tokens {
                println!("{}", tok);
            }
        }
        Err(e) => eprintln!("Lexer error: {}", e),
    }
}

fn show_ast(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot read '{}': {}", path, e);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(program) => println!("{:#?}", program),
        Err(e) => eprintln!("Parser error: {}", e),
    }
}

fn format_file(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot read '{}': {}", path, e);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(program) => {
            let mut formatter = Formatter::new();
            let formatted = formatter.format_program(&program);
            println!("{}", formatted);
        }
        Err(e) => eprintln!("Parser error: {}", e),
    }
}

fn new_project(name: &str) {
    // Create <name>/ directory
    if let Err(e) = fs::create_dir(name) {
        eprintln!("\x1b[31merror:\x1b[0m Cannot create directory '{}': {}", name, e);
        std::process::exit(1);
    }

    // Create <name>/src/ directory
    let src_dir = format!("{}/src", name);
    if let Err(e) = fs::create_dir(&src_dir) {
        eprintln!("\x1b[31merror:\x1b[0m Cannot create directory '{}': {}", src_dir, e);
        std::process::exit(1);
    }

    // Create <name>/src/main.vn with hello world template
    let main_vn_content = r#"fn main() {
    println("Hello, World!")
}
"#;
    let main_vn_path = format!("{}/main.vn", src_dir);
    if let Err(e) = fs::write(&main_vn_path, main_vn_content) {
        eprintln!("\x1b[31merror:\x1b[0m Cannot write file '{}': {}", main_vn_path, e);
        std::process::exit(1);
    }

    // Create <name>/vryn.toml with project metadata
    let toml_content = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2024"
"#, name);
    let toml_path = format!("{}/vryn.toml", name);
    if let Err(e) = fs::write(&toml_path, toml_content) {
        eprintln!("\x1b[31merror:\x1b[0m Cannot write file '{}': {}", toml_path, e);
        std::process::exit(1);
    }

    println!("\x1b[32m✓\x1b[0m Created new Vryn project '{}'", name);
    println!("  Run with: vryn run {}/src/main.vn", name);
}

fn init_project() {
    // Create src/ directory if not exists
    if !std::path::Path::new("src").exists() {
        if let Err(e) = fs::create_dir("src") {
            eprintln!("\x1b[31merror:\x1b[0m Cannot create directory 'src': {}", e);
            std::process::exit(1);
        }
    }

    // Create src/main.vn if not exists
    let main_vn = "src/main.vn";
    if !std::path::Path::new(main_vn).exists() {
        let main_content = r#"fn main() {
    println("Hello, World!")
}
"#;
        if let Err(e) = fs::write(main_vn, main_content) {
            eprintln!("\x1b[31merror:\x1b[0m Cannot write file '{}': {}", main_vn, e);
            std::process::exit(1);
        }
        println!("\x1b[32m✓\x1b[0m Created {}", main_vn);
    }

    // Create vryn.toml if not exists
    let toml_file = "vryn.toml";
    if !std::path::Path::new(toml_file).exists() {
        let toml_content = r#"[package]
name = "myproject"
version = "0.1.0"
edition = "2024"
"#;
        if let Err(e) = fs::write(toml_file, toml_content) {
            eprintln!("\x1b[31merror:\x1b[0m Cannot write file '{}': {}", toml_file, e);
            std::process::exit(1);
        }
        println!("\x1b[32m✓\x1b[0m Created {}", toml_file);
    } else {
        println!("\x1b[32m✓\x1b[0m {} already exists", toml_file);
    }

    println!("\x1b[32m✓\x1b[0m Project initialized!");
}

fn test_file(path: Option<&str>) {
    let files_to_test = if let Some(p) = path {
        vec![p.to_string()]
    } else {
        // Find all .vn files in src/
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir("src") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("vn") {
                    if let Some(path_str) = path.to_str() {
                        files.push(path_str.to_string());
                    }
                }
            }
        }
        files
    };

    let mut passed = 0;
    let mut failed = 0;

    for file_path in files_to_test {
        let source = match fs::read_to_string(&file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("\x1b[31merror:\x1b[0m Cannot read file '{}': {}", file_path, e);
                std::process::exit(1);
            }
        };

        let mut lexer = Lexer::new(&source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("\x1b[31merror[Lexer]:\x1b[0m {}", e);
                std::process::exit(1);
            }
        };

        let mut parser = Parser::new(tokens);
        let program = match parser.parse() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("\x1b[31merror[Parser]:\x1b[0m {}", e);
                std::process::exit(1);
            }
        };

        // Find test functions (ones starting with test_)
        for stmt in &program.statements {
            if let parser::ast::Statement::Function { name, params: _, body, .. } = stmt {
                if name.starts_with("test_") {
                    let mut interpreter = Interpreter::new();
                    
                    // Execute the test function
                    let test_program = parser::ast::Program {
                        statements: body.clone(),
                    };
                    
                    match interpreter.run(&test_program) {
                        Ok(_) => {
                            println!("\x1b[32m✓\x1b[0m {}", name);
                            passed += 1;
                        }
                        Err(e) => {
                            println!("\x1b[31m✗\x1b[0m {} — {}", name, e);
                            failed += 1;
                        }
                    }
                }
            }
        }
    }

    let total = passed + failed;
    println!();
    if failed == 0 {
        println!("\x1b[32mAll {} tests passed!\x1b[0m", total);
    } else {
        println!("\x1b[31m{} passed, {} failed out of {} tests\x1b[0m", passed, failed, total);
        std::process::exit(1);
    }
}


fn compile_file(filepath: &str) {
    let content = match fs::read_to_string(filepath) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m Could not read file '{}': {}", filepath, e);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&content);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("\x1b[31merror[Lexer]:\x1b[0m {}", e);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("\x1b[31merror[Parser]:\x1b[0m {}", e);
            std::process::exit(1);
        }
    };

    // Generate C code
    let mut codegen = ccodegen::CCodeGen::new();
    let c_code = codegen.generate(&program);

    // Write to .c file
    let c_filepath = filepath.replace(".vn", ".c");
    match fs::write(&c_filepath, &c_code) {
        Ok(_) => {
            println!("\x1b[32m✓\x1b[0m Generated C code in '{}'", c_filepath);
        }
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m Could not write to file '{}': {}", c_filepath, e);
            std::process::exit(1);
        }
    }

    // Optionally try to compile with gcc if available
    let status = std::process::Command::new("gcc")
        .arg(&c_filepath)
        .arg("-o")
        .arg(filepath.replace(".vn", ""))
        .status();

    match status {
        Ok(status) if status.success() => {
            println!("\x1b[32m✓\x1b[0m Compiled with gcc");
        }
        _ => {
            eprintln!("\x1b[33mwarning:\x1b[0m Could not compile with gcc (is it installed?)");
        }
    }
}

fn generate_docs(filepath: &str) {
    let content = match fs::read_to_string(filepath) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[31merror:[0m Could not read file '{}': {}", filepath, e);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&content);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[31merror[Lexer]:[0m {}", e);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[31merror[Parser]:[0m {}", e);
            std::process::exit(1);
        }
    };

    // Extract documentation
    let docs = docgen::DocGenerator::extract_docs(&program);

    // Generate HTML
    let html = docgen::DocGenerator::generate_html(&docs);

    // Write to .html file
    let html_filepath = filepath.replace(".vn", "_docs.html");
    match fs::write(&html_filepath, &html) {
        Ok(_) => {
            println!("[32m✓[0m Generated documentation in '{}'", html_filepath);
        }
        Err(e) => {
            eprintln!("[31merror:[0m Could not write to file '{}': {}", html_filepath, e);
            std::process::exit(1);
        }
    }

    // Also generate Markdown
    let markdown = docgen::DocGenerator::generate_markdown(&docs);
    let md_filepath = filepath.replace(".vn", "_docs.md");
    match fs::write(&md_filepath, &markdown) {
        Ok(_) => {
            println!("[32m✓[0m Generated documentation in '{}'", md_filepath);
        }
        Err(e) => {
            eprintln!("[31merror:[0m Could not write to file '{}': {}", md_filepath, e);
            std::process::exit(1);
        }
    }
}

fn analyze_file(filepath: &str) {
    let content = match fs::read_to_string(filepath) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m Could not read file '{}': {}", filepath, e);
            std::process::exit(1);
        }
    };

    let result = VrynAnalyzer::analyze(&content);

    // Print diagnostics
    if !result.diagnostics.is_empty() {
        println!("\x1b[33mDiagnostics:\x1b[0m");
        for diag in &result.diagnostics {
            let color = match diag.severity {
                lsp::DiagnosticSeverity::Error => "\x1b[31m",
                lsp::DiagnosticSeverity::Warning => "\x1b[33m",
                lsp::DiagnosticSeverity::Info => "\x1b[36m",
            };
            let reset = "\x1b[0m";
            println!(
                "  {}{}{}:{}: {} - {}",
                color,
                diag.severity,
                reset,
                diag.line,
                diag.column,
                diag.message
            );
        }
    } else {
        println!("\x1b[32m✓ No diagnostics\x1b[0m");
    }

    println!();

    // Print symbols
    if !result.symbols.is_empty() {
        println!("\x1b[33mSymbols:\x1b[0m");
        for symbol in &result.symbols {
            let kind_str = symbol.kind.to_string();
            if let Some(type_info) = &symbol.type_info {
                println!("  {} {} : {} (line {})", kind_str, symbol.name, type_info, symbol.line);
            } else {
                println!("  {} {} (line {})", kind_str, symbol.name, symbol.line);
            }
        }
    } else if result.success {
        println!("\x1b[33mSymbols:\x1b[0m No symbols found");
    }

    println!();
    println!("\x1b[33mAnalysis Result:\x1b[0m {}", if result.success { "Success" } else { "Failed" });
}
