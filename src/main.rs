mod lexer;
mod parser;
mod codegen;
mod typechecker;

use std::env;
use std::fs;
use std::io::{self, Write, BufRead};

use lexer::Lexer;
use parser::Parser;
use codegen::Interpreter;
use typechecker::TypeChecker;

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
    repl              Start interactive REPL
    check <file.vn>   Type-check without running
    version           Show version info
    help              Show this help message

  DEBUG:
    tokens <file.vn>  Show lexer output (tokens)
    ast <file.vn>     Show parser output (AST)

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
