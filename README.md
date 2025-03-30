# SWC C++ Bindings

High-performance TypeScript-to-JavaScript transpiler built with SWC (Rust) and exposed as C-compatible shared library.

## Features
- ⚡ **Blazing Fast** - Transpiles TS in microseconds
- 🔗 **C/C++ Compatible** - Simple C API via shared library
- 🧩 **Zero Runtime** - No Node.js/TypeScript toolchain required

## Quick Start

### Prerequisites
- Rust 1.65+
- C++17 compiler
- cbindgen: `cargo install cbindgen`

```bash
git clone https://github.com/jibbex/swc-cxx-bindings.git
cd swc-cxx-bindings
```

### Build & Use
```bash
# Build library and generate header
cargo build --release

# Use in C++ (see example.cpp)
g++ -std=c++17 examples/example.cpp -I. -L./target/release -lswc_ffi -o examples/example
```

## API
```c
// Transpiles TypeScript to JavaScript
// Caller must free returned string with swc_free_string()
char* swc_transpile_ts(const char* typescript_code);

// Frees memory allocated by swc_transpile_ts()
void swc_free_string(char* str);
```

## Performance (i9-13900K)
| Operation          | Time    |
|--------------------|---------|
| 100 LOC TS         | 8μs     |
| 10,000 LOC TS      | 1.2ms   |

## License
MIT