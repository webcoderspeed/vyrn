# Vryn Programming Language

> **"C ki speed, Python ki simplicity, Rust ki safety, Java ki universality"**

Vryn is a new, general-purpose programming language designed to be fast, safe, and easy to use. It combines the performance and safety of systems programming languages like Rust with the developer experience and simplicity of high-level languages like Python.

🚧 **Status:** Alpha (v0.1.0) — Under active development.

## 🚀 Vision

Vryn aims to solve the "two-language problem" where developers prototype in Python/JS but rewrite in C++/Rust for performance. Vryn is designed to be:

*   **Fast:** Compiles to efficient machine code (via LLVM/Wasm in future, currently interpreted).
*   **Safe:** Memory safety without garbage collection (ownership model).
*   **Simple:** Clean, readable syntax with minimal boilerplate.
*   **Universal:** Suitable for systems programming, web servers, scripts, and more.

## ✨ Features

*   **Modern Syntax:** Clean, expression-based syntax inspired by Rust and Python.
*   **Rich Type System:** Static typing with type inference, Structs, Enums, and Traits.
*   **Pattern Matching:** Powerful `match` expressions for control flow.
*   **Tooling First:** Built-in formatter, linter, test runner, and language server.
*   **Memory Safety:** Ownership and borrowing rules (in progress).
*   **No GC:** Deterministic resource management.

## 📦 Installation

To build Vryn from source, you need **Rust** (cargo) installed.

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/webcoderspeed/vryn.git
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

Vryn comes with a comprehensive CLI tool.

### Project Management
Initialize a new project or create one from scratch:
```bash
vryn new my_project
# or
vryn init
```

### Running Code
Execute a Vryn source file or start the REPL:
```bash
# Run a file
vryn run examples/hello.vn

# Start Interactive REPL
vryn repl
```

### Development Tools
Vryn includes built-in tools for a better developer experience:

```bash
# Check syntax and errors without running
vryn check examples/hello.vn

# Format code
vryn fmt examples/hello.vn

# Run tests
vryn test
vryn test examples/my_test.vn

# Analyze code (LSP mode)
vryn analyze examples/hello.vn
```

### Debugging
View the internal representation of your code:
```bash
# View Tokens
vryn tokens examples/hello.vn

# View Abstract Syntax Tree (AST)
vryn ast examples/hello.vn
```

## 📖 Syntax Examples

### Hello World
```vryn
fn main() {
    println("Hello, World!")
}
```

### Variables & Data Types
```vryn
let name = "Vryn"       // Type inferred as str
let count: int = 42     // Explicit typing
let pi = 3.14
let is_fast = true
```

### Structs & Enums
```vryn
struct User {
    username: str,
    active: bool,
}

enum Status {
    Pending,
    Active,
    Suspended(str), // Variant with payload
}

let user = User {
    username: "dev",
    active: true,
}
```

### Pattern Matching
```vryn
let status = Status::Suspended("Spam")

match status {
    Status::Active => println("User is active"),
    Status::Suspended(reason) => println("Suspended: " + reason),
    _ => println("Status unknown"),
}
```

### Functions
```vryn
fn add(a: int, b: int) -> int {
    return a + b
}

// Lambda / Closure
let multiply = |x, y| x * y
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details on how to get started.

Please adhere to our [Code of Conduct](CODE_OF_CONDUCT.md) in all interactions.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Community

*   **Issues:** [GitHub Issues](https://github.com/webcoderspeed/vryn/issues)
*   **Discussions:** [GitHub Discussions](https://github.com/webcoderspeed/vryn/discussions)

---
*Built with ❤️ in Rust.*
