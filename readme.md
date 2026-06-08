[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Progress](https://img.shields.io/badge/Lox-85%25-yellow.svg)]()

# Lox Interpreter

`lox` is a bytecode virtual machine implementation of the Lox programming language from the book "Crafting Interpreters" by Robert Nystrom.

## Table of Contents

- [Description](#description)
- [Features](#features)
- [Plans for future](#plans-for-future)
- [Installation](#installation)
- [Usage](#usage)
- [Examples](#examples)
- [Implementation Details](#implementation-details)
- [Limitations](#limitations)
- [How it works](#how-it-works)
- [Contributing](#contributing)
- [Acknowledgments](#acknowledgments)
- [License](#license)

## Description

**Lox Interpreter** is a fully functional implementation of the Lox language, featuring a custom bytecode compiler, stack-based virtual machine, and automatic memory management through a mark-sweep garbage collector.

This project follows the implementation from Robert Nystrom's excellent book, with additional enhancements including:
- Complete mark-sweep garbage collection with tri-color abstraction
- String interning and weak reference handling
- Support for closures and upvalues
- REPL environment for interactive development
- Production-ready memory management

## Features

### Core Language
- **Variables** - Mutable variables with `var` keyword
- **Control Flow** - `if`/`else` conditional statements
- **Loops** - `while` and `for` loops with `break` and `continue`
- **Functions** - First-class functions with closures
- **Scope** - Block-scoped variables with proper lexical scoping

### Types
- **Numbers** - 64-bit floating point numbers
- **Strings** - Immutable strings with interning
- **Booleans** - `true` and `false` literals
- **Nil** - Null/none value

### Runtime
- **Bytecode Compiler** - Compiles source to efficient bytecode
- **Stack-based VM** - Fast execution with minimal overhead
- **Garbage Collector** - Mark-sweep collector with automatic heap management
- **Native Functions** - Rust functions that can be called from Lox code
- **REPL** - Interactive read-eval-print loop for testing

### Technical Features
- **Borrow Checker Friendly** - Safe Rust implementation with minimal `unsafe`
- **No Dependencies** - Pure Rust implementation (except `rand` for native functions)
- **Memory Efficient** - ~100MB stable memory usage under heavy allocation
- **Self-tuning GC** - Adaptive collection frequency based on heap size

## Plans for future

### Short term (Next 1-2 months)
- [ ] **Classes and Objects** - OOP support with methods, fields, and constructors
- [ ] **Inheritance** - Single inheritance with `super` keyword
- [ ] **Method resolution** - Proper method lookup chain
- [ ] **Full long constant support** - Allow to declare more than 256 consts
### Mid term (3-6 months)
- [ ] **Arrays** - List data structure with index access and built-in methods
- [ ] **Ternary operator `? :` - allow to use bool ? {} : {}
- [ ] **Switch statements** - Add support to switch keyword
- [ ] **Increment/Decrement operators** - add support for `++`, '--' left and right side versions
- [ ] **Modify and set operators** - Add support for `+=`, `-=`, etc.
- [ ] **Maps/Dictionaries** - Key-value storage with string keys
- [ ] **Exception Handling** - Try-catch-finally blocks for error recovery
- [ ] **Module System** - Import/export between multiple files

### Long term (6+ months)
- [ ] **JIT Compilation** - Performance optimization for hot code paths
- [ ] **Debugger** - Step-through execution with breakpoints and variable inspection
- [ ] **Concurrency** - Lightweight threads or async/await support
- [ ] **Standard Library** - Collections, I/O, networking, and more
- [ ] **Self-hosting** - Rewrite compiler in Lox itself

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
lox = { git = "https://github.com/Grapple228/lox_interpreter" }
```

Or clone and build locally:
```sh
git clone https://github.com/yourusername/lox_interpreter
cd lox_interpreter
cargo build --release
```

## Usage

Run a file
```sh
cargo run -- path/to/your/script.lox
```

Interactive REPL
```sh
cargo run
```

Run with debug messages
```sh
RUST_LOG=debug cargo run -- script.lox
```

## Examples
Fibonacci (Recursive)
```rust
fun fib_rec(n) {
    if (n < 2) return n;
    return fib_rec(n - 1) + fib_rec(n - 2);
}

print fib_rec(20);
```

Fibonacci (Iterative)
```rust
fun fib_iter(n) {
    var a = 0;
    var b = 1;
    
    for (var i = 0; i < n; i = i + 1) {
        var temp = a + b;
        a = b;
        b = temp;
    }
    
    return a;
}

print fib_iter(20);
```

Closures
```rust
fun makeCounter() {
    var count = 0;
    fun counter() {
        count = count + 1;
        return count;
    }
    return counter;
}

var counter = makeCounter();
print counter();  // 1
print counter();  // 2
```

String concatenation with type coercion
```rust
var greeting = "Hello";
var name = "World";
print greeting + " " + name;  // "Hello World"

print "The answer is " + 42;   // "The answer is 42"
print true + " is true";        // "true is true"
```

Loops with break and continue
```rust
for (var i = 0; i < 10; i = i + 1) {
    if (i % 2 == 0) continue;
    if (i > 7) break;
    print i;  // 1, 3, 5, 7
}
```

Native functions
```rust
var start = clock();
// ... do work ...
print "Took: " + (clock() - start);

print random(1, 100);  // Random number between 1 and 100
print square(12);      // 144
```

## Implementation Details

### Architecture
The interpreter consists of four main components:

1. **Scanner** - Tokenizes source code into tokens
2. **Compiler** - Compiles tokens to bytecode with constant folding and jump patching
3. **VM** - Executes bytecode instructions on a stack-based virtual machine
4. **GC** - Mark-sweep garbage collector managing heap-allocated objects

### Memory Layout
- **Value Stack** - 16,384 slots for temporary values and locals
- **Call Frames** - 64 frames maximum for function calls
- **Heap** - Dynamic memory for strings, functions, closures, and upvalues

### Garbage Collection
- **Tricolor Abstraction** - White (unreached), Gray (worklist), Black (processed)
- **Weak References** - String interning table doesn't prevent collection
- **Self-tuning** - GC threshold adjusts based on live heap size
- **Roots** - Stack, globals, call frames, open upvalues, compiler chain

## Limitations

- No classes or inheritance (coming soon)
- No array or map data structures
- No exception handling
- Single-threaded execution only
- 64 call frames maximum

## Troubleshooting
Q: Memory keeps growing slowly
A: This is normal - GC runs when heap exceeds threshold. Adjust next_gc in VM initialization for more aggressive collection.

Q: Stack overflow error
A: Maximum call depth is 64 frames. Refactor recursive functions to be iterative.

Q: Compilation error with native functions
A: Ensure native functions are registered in init_natives() before compilation.

## How it works
Source Code → Scanner → Tokens → Compiler → Bytecode → VM → Result
↓
Constant Table
Chunk (instructions)
Debug info (line numbers)

The VM executes bytecode in a loop:
1. Fetch instruction at current IP
2. Decode opcode and operands
3. Execute operation (push/pop, arithmetic, control flow, etc.)
4. Increment IP and repeat

### Contributing
These features are open for contribution! See [Contributing](#contributing) section for details.

## Acknowledgments

- **Robert Nystrom** - For the excellent ["Crafting Interpreters"](https://craftinginterpreters.com/) book
- **The Rust Community** - For making systems programming safe and enjoyable
- **Open Source Contributors** - Your pull requests help make this project better

This implementation follows the book's design but adds:
- Complete GC implementation with weak references
- String interning optimization
- `continue` and `break` keywords in loops
- Better error handling and reporting
- Binary `%` (modulo) operator
- String coercion with `+` operator
- `is_compiling` guard - Prevents GC from running during compilation, avoiding crashes
- Constant caching - Identifier constants are cached to avoid duplicate string allocations
- Long constants support - Support for 24-bit constant indices (OP_CONSTANT_LONG)
- Custom stack implementation - Type-safe stack with better ergonomics than C version
- Debug logging - Optional GC and execution tracing via RUST_LOG

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.


--- 

*Built with ❤️ in Rust*
