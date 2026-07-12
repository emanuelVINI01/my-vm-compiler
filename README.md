<div align="center">
  <h1>⚙️ my-vm-compiler (CVM)</h1>
  <p>
    <strong>A robust, custom compiler for the CVM language, targeting a custom virtual machine architecture.</strong>
  </p>
  <p>
    <a href="https://github.com/emanuelVINI01/my-vm-compiler/actions"><img src="https://img.shields.io/badge/build-passing-brightgreen?style=flat-square" alt="Build Status"></a>
    <a href="https://doc.rust-lang.org/cargo/"><img src="https://img.shields.io/badge/cargo-package-orange?style=flat-square" alt="Cargo"></a>
    <a href="https://github.com/emanuelVINI01/my-vm-compiler/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License"></a>
    <a href="https://rust-lang.org"><img src="https://img.shields.io/badge/rust-2024-black?style=flat-square&logo=rust" alt="Rust Version"></a>
  </p>
</div>

## 📖 Overview

**my-vm-compiler** is a custom language compiler written in Rust using [Pest](https://pest.rs/) for PEG parsing. It transforms `*.cvm` language files into an intermediate representation (IR), applies optimization passes, and ultimately generates Assembly (`*.asm`) code intended for a bespoke Virtual Machine (VM) architecture.

## ✨ Features

- **Robust Syntax**: C-like syntax with structs, arrays, pointers, and a strong typing system (`int`, `float`, `string`, `bool`, `void`).
- **Control Flow**: Supports `if`, `while`, and `for` loop constructs.
- **Advanced Types & Structs**: Direct support for field access and struct declarations.
- **Inline Assembly**: Seamless integration with the target architecture using `asm { "..." }` blocks.
- **Macro System**: Support for macro invocation syntax (e.g., `macro_name!(args)`).
- **Import Resolution**: Modular source code organization through `import "file.cvm";`.
- **Interrupts**: Native support for hardware/software interrupt handler routines.
- **IR Generation & Optimization**: Built-in compilation passes emitting an Intermediate Representation prior to code generation.

## 🚀 Getting Started

### Prerequisites

Ensure you have [Rust & Cargo](https://rustup.rs/) installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Building from Source

1. Clone the repository:
   ```bash
   git clone https://github.com/emanuelVINI01/my-vm-compiler.git
   cd my-vm-compiler
   ```
2. Build the project:
   ```bash
   cargo build --release
   ```

## 🛠️ Usage

Use the compiled binary to compile your `.cvm` programs into Assembly:

```bash
cargo run -- <input.cvm> <output.asm>
```

Or using the release binary directly:

```bash
./target/release/my-vm-compiler program.cvm program.asm
```

### Output Files

The compiler will produce two artifacts during compilation:
1. `output.asm`: The final generated assembly file.
2. `input.ir`: An Intermediate Representation dump file (useful for debugging).

## 📝 Syntax Snippets

Here is a quick look at the CVM language syntax:

```c
// Variable Declarations
int x = 10;
float pi = 3.14;
string msg = "Hello, VM!";
bool is_ready = true;

// Pointers and Arrays
int[5] numbers;
*int ptr = &x;

// Structs
struct Point {
    int x;
    int y;
}

// Control Flow
if (x > 5) {
    x++;
} else {
    x--;
}

while (x > 0) {
    x--;
}

// Functions & Interrupts
int calculate(int a, int b) {
    return a + b;
}

interrupt void handle_timer() {
    // Interrupt handling logic
}

// Inline Assembly
asm { "MOV R1, 10" }
```

## 🏗️ Architecture

1. **Lexer & Parser**: Powered by `pest` (`src/grammar.pest`).
2. **IR Generator**: Traverses the AST and generates an unoptimized Intermediate Representation (`src/generator`).
3. **Optimizer**: Runs passes over the IR to optimize instructions (`src/optimizer`).
4. **Code Generator**: Converts optimized IR into the target Assembly language (`src/codegen`).

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
