# 🚀 SWC C++ Bindings

> *Because sometimes we need to wrap Rust tools in C++ which we'll later call from JavaScript... programmers, right?* 🤦‍♂️

TypeScript transpiler for C++ - brought to you by someone who thought "you know what this Rust-based tool needs? More languages!" 
These bindings allow you to use SWC's lightning-fast TS transpiler functionality directly in C++, without having to install Node.js 
on your beautifully organized machine (that probably has 47 versions of Node anyway).

**⚠️ Note:** This project is like my attempts at baking - experimental and not recommended if you want predictable results.

## ✨ Features

- ⚡ **Absurdly Fast** - Transpiles TypeScript faster than you can say "why am I wrapping a Rust tool in C++?"
- 🔗 **C/C++ Compatible** - Simple C API that lets you pretend you're not actually using Rust under the hood
- 🧩 **Zero Runtime** - No Node.js or TypeScript toolchain needed (one less dependency to break your build!)
- 🦀 **Powered by Rust** - Yes, we're using a Rust-based tool, wrapping it in C++, to process JavaScript... software engineering at its finest!

## 🚦 Quick Start

### Prerequisites

- Rust 1.65+ (essential since SWC is written in Rust - the language we're wrapping but pretending not to use)
- C++17 compiler (because every project needs at least three languages involved)
- cbindgen: `cargo install cbindgen` (perfect for those who enjoy installing tools to build tools)

```bash
# Clone the repo (and question your life choices)
git clone https://github.com/jibbex/swc_cxx_bindings.git
cd swc_cxx_bindings
```

### Build & Use

```bash
# Build library and generate header (time to pretend you understand Cargo)
cargo build --release

# Use in C++ (while secretly thanking the Rust developers)
g++ -std=c++17 examples/example.cpp -I. -L. -lswc -Wl,-rpath,'$ORIGIN/..' -o examples/example

# Run the example (and wonder why you're using C++ bindings for a Rust tool)
examples/example examples/example.ts

# On Windows? May the force be with you! 🍀
```

## 🔧 API

```c
// Transpiles TypeScript to JavaScript by secretly calling Rust
// Don't forget to free the string or you'll create the world's slowest memory leak!
char* swc_transpile_ts(const char* typescript_code);

// Frees the memory (the most important function that everyone forgets to call)
void swc_free_string(char* str);
```

## ⚡ Performance (because we need to justify this complexity)

| Operation          | Time      | What You Could Be Doing Instead |
|--------------------|-----------|--------------------------------|
| 100 lines of TS    | 8μs       | Absolutely nothing measurable  |
| 10,000 lines of TS | 1.2ms     | Contemplate why you're using C++ bindings for a Rust tool |
| Starting Node.js   | 300ms     | Write a blog post about why Rust is better than both C++ and JavaScript |

## 📝 Example

```cpp
#include "swc_ffi.h"
#include <iostream>

int main() {
    const char* ts_code = "const hello: string = 'Hello, World!';";
    
    // Here we call Rust from C++ to process JavaScript. What a time to be alive!
    char* js_code = swc_transpile_ts(ts_code);
    
    std::cout << "Input TypeScript: " << ts_code << std::endl;
    std::cout << "Output JavaScript: " << js_code << std::endl;
    
    // Memory management - the thing we're doing manually that Rust would handle for us
    swc_free_string(js_code);
    
    return 0;
}
```

## 📋 ToDo

- [ ] Write more tests (bookmark this for your "someday" folder)
- [ ] Better error messages (instead of the classic "something went wrong somewhere")
- [ ] Explain to your team why you're using C++ bindings for a Rust tool to process JavaScript
- [ ] CMake integration (because we clearly don't have enough build systems involved yet)

## 📜 License

MIT - Use at your own risk, and maybe question why you need C++ bindings for a Rust tool in the first place!
