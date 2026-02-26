# Vryn Programming Language

> **"C ki speed, Python ki simplicity, Rust ki safety, Java ki universality"**

Vryn is a new, general-purpose programming language designed to be fast, safe, and easy to use. It combines the performance and safety of systems programming languages like Rust with the developer experience and simplicity of high-level languages like Python.

🚧 **Status:** Pre-Alpha (v0.1.0-alpha) — Under active development.

## 🚀 Vision

Vryn aims to solve the "two-language problem" where developers prototype in Python/JS but rewrite in C++/Rust for performance. Vryn is designed to be:

*   **Fast:** Compiles to efficient machine code (via LLVM in future, currently interpreted).
*   **Safe:** Memory safety without garbage collection (ownership model).
*   **Simple:** Clean, readable syntax with minimal boilerplate.
*   **Universal:** Suitable for systems programming, web servers, scripts, and more.

## ✨ Features (Implemented so far)

*   **Modern Syntax:** Clean, expression-based syntax inspired by Rust and Python.
*   **Lexer & Parser:** Full recursive descent parser with error recovery.
*   **Interpreter:** Tree-walking interpreter for immediate feedback (REPL & Run).
*   **Type System (Partial):** Static typing with type inference (work in progress).
*   **Control Flow:** `if/else`, `while`, `for` loops, and `match` expressions.
*   **Functions:** First-class functions with lexical scoping.
*   **Data Structures:** Arrays, Structs (parsing only), and basic primitives.
*   **Tooling:** Built-in CLI for running, checking, and debugging code.

## 📦 Installation

To build Vryn from source, you need **Rust** installed.

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/vryn-lang/vryn.git
    cd vryn
    ```

2.  **Build the project:**
    ```bash
    cargo build --release
    ```

3.  **Run the CLI:**
    ```bash
    ./target/release/vryn --help
    ```

    *Tip: Add `./target/release/` to your PATH to use `vryn` globally.*

## 💻 Usage

### Interactive REPL
Start the Read-Eval-Print Loop to experiment with Vryn syntax:
```bash
vryn repl
```

### Run a File
Execute a Vryn source file (`.vn`):
```bash
vryn run examples/hello.vn
```

### Check Syntax
Verify code without running it:
```bash
vryn check examples/hello.vn
```

### Debugging
View the internal tokens or AST:
```bash
vryn tokens examples/hello.vn
vryn ast examples/hello.vn
```

## 📖 Syntax Examples

### Hello World
```vryn
fn main() {
    println("Hello, World!")
}
```

### Variables & Math
```vryn
let name = "Vryn"       // Type inferred as str
let age: i32 = 1        // Explicit type annotation

let x = 10
let y = 20
println("Sum: " + (x + y))
```

### Functions
```vryn
fn add(a: i32, b: i32) -> i32 {
    return a + b
}

let result = add(5, 10)
println(result)
```

### Control Flow
```vryn
if age >= 18 {
    println("Adult")
} else {
    println("Minor")
}

// Loops
let mut i = 0
while i < 5 {
    println(i)
    i = i + 1
}

// For loop
for x in 0..10 {
    println(x)
}
```

### Arrays
```vryn
let numbers = [1, 2, 3, 4, 5]
println(numbers[0]) // Access by index
```

## 🗺️ Roadmap

*   **Phase 0:** Design & Specification ✅
*   **Phase 1:** Core Compiler & Interpreter (MVP) 🚧 *(Current)*
*   **Phase 2:** Standard Library (File I/O, Networking)
*   **Phase 3:** Tooling (LSP, Formatter, Package Manager)
*   **Phase 4:** Self-Hosting (Rewriting Vryn in Vryn)

## 🤝 Contributing

Contributions are welcome! If you're interested in language design or compiler engineering:

1.  Check the [Issues](https://github.com/vryn-lang/vryn/issues) for open tasks.
2.  Read `CONTRIBUTING.md` (coming soon).
3.  Fork the repo and submit a PR!

## 📄 License

This project is licensed under the **MIT License**.
