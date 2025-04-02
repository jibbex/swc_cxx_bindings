/// # swc_ffi - FFI bindings for swc
///
/// An C/C++ Interop for the swc Rust library.
use std::{
    ffi::{CStr, CString},
    os::raw::c_char
};
use std::path::Path;
use std::sync::Arc;
use swc::try_with_handler;
use swc_common::{errors::{ColorConfig, Handler}, sync::Lrc, FileName, Globals, Mark, SourceFile, SourceMap, GLOBALS};
use swc_common::comments::SingleThreadedComments;
use swc_ecma_codegen::{Config, Emitter};
use swc_ecma_transforms_react::{jsx, Options as JsxOptions};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::hygiene::hygiene;
use swc_ecma_ast::Pass;
use swc_ecma_transforms_base::resolver;
use swc_ecma_transforms_typescript::strip;
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_visit::VisitMutWith;
use anyhow::{Context, Error};

/// Represents a file to transpile
///
/// This enum is used to represent a file to transpile. It can either be a file path or a file name with its content.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use swc_ffi::File;
///
/// let path = Path::new("test.ts");
/// let fileFromPath = File::FilePath(path);
///
/// let fileFromSource = File::FileName("test.ts", "const a = 1;".to_string());
/// ```
///
pub enum File {
    FilePath(&'static Path),
    FileName(FileName, String),
}

/// Transpile TypeScript/TSX to JavaScript
///
/// This function transpiles TypeScript/TSX code to JavaScript.
///
/// # Arguments
///
/// * `cm` - The source map
/// * `filename` - The file to transpile
///
/// # Returns
///
/// The transpiled JavaScript code as a string
///
/// # Errors
///
/// This function returns an error if the transpilation fails. The error is a boxed trait
/// object that implements the `Error` trait.
///
/// # Examples
///
/// ```rust
/// use swc_common::sync::Lrc;
/// use swc_common::SourceMap;
/// use swc_ffi::{transpile_tsx_to_js, File};
/// use std::path::Path;
///
/// let cm: Lrc<SourceMap> = Default::default();
/// let filename = File::FilePath(Path::new("test.ts"));
/// let result = transpile_tsx_to_js(cm, filename);
///
/// match result {
///     Ok(output) => {
///         println!("{}", output);
///     },
///     Err(e) => {
///         eprintln!("Error: {}", e);
///     }
/// }
/// ```
///
/// # Panics
///
/// This function panics if the program parsing fails. This should never happen in practice.
pub fn transpile_tsx_to_js(
    cm: Lrc<SourceMap>,
    filename: File,
) -> Result<String, Box<dyn std::error::Error>> {
    let handler = Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(cm.clone())
    );

    // Load or create file
    let fm = get_js_file(filename, cm.clone())?;
    let comments = SingleThreadedComments::default();

    // Configure parser for TypeScript/TSX
    let syntax = Syntax::Typescript(TsSyntax {
        tsx: fm.name.to_string().ends_with(".tsx"),
        decorators: true,
        dts: false,
        no_early_errors: false,
        disallow_ambiguous_jsx_like: false,
    });

    // Create lexer and parser
    let lexer = Lexer::new(
        syntax,
        Default::default(),
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);

    // Parse the program
    let mut program = parser
        .parse_program()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect("program parsing failed");

    let globals = Globals::default();
    GLOBALS.set(&globals, || {
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();
        let mut config: Config = Default::default();
        let jsx_options = JsxOptions {
            pragma: Some(Lrc::new("React.createElement".into())),
            pragma_frag: Some(Lrc::new("React.Fragment".into())),
            ..Default::default()
        };

        config.target = swc_ecma_ast::EsVersion::Es2015;
        program.visit_mut_with(&mut resolver(unresolved_mark, top_level_mark, true));
        program.visit_mut_with(&mut jsx(cm.clone(), Some(&comments), jsx_options, top_level_mark, unresolved_mark));
        strip(unresolved_mark, top_level_mark).process(&mut program);
        program.visit_mut_with(&mut hygiene());
        program.visit_mut_with(&mut fixer(Some(&comments)));
        let mut buf = vec![];
        let mut emitter = Emitter {
            cfg: config,
            cm: cm.clone(),
            comments: Some(&comments),
            wr: Box::new(JsWriter::new(
                cm.clone(),
                "\n",
                &mut buf,
                None,
            )),
        };

        emitter.emit_program(&program)?;

        Ok(String::from_utf8(buf)?)
    })
}

/// Compiles a TypeScript/JavaScript file to JavaScript using SWC.
///
/// This internal function handles the compilation process for both file-based and
/// string-based input sources.
///
/// # Parameters
///
/// * `file` - The source file representation (either a file path or in-memory content)
///
/// # Returns
///
/// * `Some(String)` - The compiled JavaScript code on success
/// * `None` - If compilation fails for any reason
///
/// # Implementation Details
///
/// Uses the SWC compiler with default settings to transform TypeScript/TSX to JavaScript.
/// The compilation process is executed within the SWC global context.
fn compile(file: File) -> Option<String> {
    let cm: Lrc<SourceMap> = Default::default();
    let compiler = swc::Compiler::new(cm.clone());
    let output = GLOBALS
        .set(&Default::default(), || {
            try_with_handler(cm.clone(), Default::default(), |handler| {
                let fm = get_js_file(file, cm)?;
                compiler.process_js_file(fm, handler, &Default::default())
                    .context("failed to process file")
            })
        });

    match output {
        Ok(output) => Some(output.code),
        Err(_) => None
    }
}

/// Compiles a TypeScript/TSX file to JavaScript.
///
/// This function takes a path to a TypeScript or TSX file, compiles it to JavaScript,
/// and returns the compiled code as a C-compatible string. The caller owns the returned
/// string and is responsible for freeing it with `free_string()`.
///
/// # Parameters
///
/// * `filepath` - C string pointer to the path of the TypeScript/TSX file to compile
/// * `error` - Mutable reference to a C char that will contain error message if compilation fails
///
/// # Returns
///
/// * On success: Raw pointer to a null-terminated C string containing the compiled JavaScript
/// * On failure: Null pointer, with error message populated in the `error` parameter
///
/// # Safety
///
/// This function is unsafe because:
/// * It dereferences raw pointers
/// * It converts between C and Rust string representations
/// * It allocates memory that must be freed by the caller using `free_string()`
///
/// # Examples
///
/// ```c
/// char error[256] = {0};
/// char* result = compile_file("path/to/component.tsx", error);
/// if (result) {
///     // Use the compiled JavaScript
///     free_string(result); // Free when done
/// } else {
///     // Handle error
///     printf("Error: %s\n", error);
/// }
/// ```
#[no_mangle]
pub extern "C" fn compile_file(filepath: *const c_char, error: &mut c_char) -> *mut c_char {
    let path = unsafe { CStr::from_ptr(filepath) }.to_str()
        .expect("failed to convert filepath to &str");
    let file = File::FilePath(Path::new(path));
    prepare_compile_result(error, file)
}

/// Compiles TypeScript/JavaScript code provided as a string.
///
/// This function takes a C-style null-terminated string containing TypeScript or JavaScript code,
/// compiles it to JavaScript, and returns the compiled code as a C-compatible string. The caller
/// owns the returned string and is responsible for freeing it with `free_string()`.
///
/// # Parameters
///
/// * `code` - C string pointer containing the TypeScript/JavaScript code to compile
/// * `error` - Mutable reference to a C char that will contain error message if compilation fails
///
/// # Returns
///
/// * On success: Raw pointer to a null-terminated C string containing the compiled JavaScript
/// * On failure: Null pointer, with error message populated in the `error` parameter
///
/// # Safety
///
/// This function is unsafe because:
/// * It dereferences raw pointers
/// * It converts between C and Rust string representations
/// * It allocates memory that must be freed by the caller using `free_string()`
///
/// # Examples
///
/// ```c
/// char error[256] = {0};
/// char* result = compile_js("const greeting: string = 'Hello, world!';", error);
/// if (result) {
///     // Use the compiled JavaScript
///     free_string(result); // Free when done
/// } else {
///     // Handle error
///     printf("Error: %s\n", error);
/// }
/// ```
#[no_mangle]
pub extern "C" fn compile_js(code: *const c_char, error: &mut c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(code) }.to_str()
        .expect("failed to convert code to &str");
    let file = File::FileName(FileName::Custom("input.js".into()), input.into());
    prepare_compile_result(error, file)
}

/// Helper function to handle the common logic for compiling TypeScript/JavaScript.
///
/// This internal function encapsulates the shared compilation and error handling logic
/// used by both `compile_file` and `compile_js` functions.
///
/// # Parameters
///
/// * `error` - Mutable reference to a C char that will contain error message if compilation fails
/// * `file` - The source file representation (either a file path or in-memory content)
///
/// # Returns
///
/// * On success: Raw pointer to a null-terminated C string containing the compiled JavaScript
/// * On failure: Null pointer, with error message populated in the `error` parameter
///
/// # Safety
///
/// This function is unsafe because it manipulates raw pointers when setting the error message.
fn prepare_compile_result(error: &mut c_char, file: File) -> *mut c_char {
    match compile(file) {
        Some(output) => CString::new(output)
            .expect("failed to serialize code").into_raw(),
        None => {
            unsafe {
                *error = *CString::new("failed to compile file")
                    .expect("failed to convert error message to CString")
                    .into_raw()
            };
            std::ptr::null_mut()
        }
    }
}

/// Minifies JavaScript code using SWC.
///
/// This internal function handles the minification process for both file-based and
/// string-based JavaScript sources.
///
/// # Parameters
///
/// * `file` - The source file representation (either a file path or in-memory content)
///
/// # Returns
///
/// * `Some(String)` - The minified JavaScript code on success
/// * `None` - If minification fails for any reason
///
/// # Implementation Details
///
/// Uses the SWC compiler with optimized minification settings:
/// - Compression enabled (reduces code size through various optimizations)
/// - Name mangling enabled (shortens variable/function names)
/// - Uses a simple mangle cache to ensure consistent name replacements
fn minify(file: File) -> Option<String> {
    let cm: Lrc<SourceMap> = Default::default();
    let compiler = swc::Compiler::new(cm.clone());
    let output = GLOBALS
        .set(&Default::default(), || {
            try_with_handler(cm.clone(), Default::default(), |handler| {
                let fm = get_js_file(file, cm)?;
                compiler.minify(
                    fm,
                    handler,
                    &swc::config::JsMinifyOptions {
                        compress: swc::BoolOrDataConfig::from_bool(true),
                        mangle: swc::BoolOrDataConfig::from_bool(true),
                        ..Default::default()
                    },
                    swc::JsMinifyExtras::default()
                        .with_mangle_name_cache(Some(Arc::new(swc_ecma_minifier::option::SimpleMangleCache::default()))),
                )
                .context("failed to minify")
            })
        });

    match output {
        Ok(output) => Some(output.code),
        Err(_) => None
    }
}

/// Minifies JavaScript code from a file path.
///
/// This function takes a C-style null-terminated string path to a JavaScript file,
/// reads the file, minifies its contents, and returns the minified code as a
/// C-compatible string. The caller owns the returned string and is responsible
/// for freeing it with the appropriate FFI deallocation function (e.g. `free_string`,
/// `free_const_string`).
///
/// # Parameters
///
/// * `filepath` - C string pointer to the path of the JavaScript file to minify
/// * `error` - Mutable reference to a C char that will contain error message if minification fails
///
/// # Returns
///
/// * On success: Raw pointer to a null-terminated C string containing the minified JavaScript
/// * On failure: Null pointer, with error message populated in the `error` parameter
///
/// # Safety
///
/// This function is unsafe because:
/// * It dereferences raw pointers
/// * It converts between C and Rust string representations
/// * It allocates memory that must be freed by the caller
///
/// # Examples
///
/// ```c
/// char error[256] = {0};
/// char* result = minify_js_file("path/to/script.js", error);
/// if (result) {
///     // Use the minified JavaScript
///     free_string(result); // Free when done
/// } else {
///     // Handle error
///     printf("Error: %s\n", error);
/// }
/// ```
pub extern "C" fn minify_js_file(filepath: *const c_char, error: &mut c_char) -> *mut c_char {
    let path = unsafe { CStr::from_ptr(filepath) }.to_str()
        .expect("failed to convert filepath to &str");
    let file = File::FilePath(Path::new(path));
    match minify(file) {
        Some(output) => CString::new(output)
            .expect("failed to serialize code").into_raw(),
        None => {
            unsafe {
                *error = *CString::new("failed to minify file")
                    .expect("failed to convert error message to CString")
                    .into_raw()
            };
            std::ptr::null_mut()
        }
    }
}

/// Minifies JavaScript code provided as a string.
///
/// This function takes a C-style null-terminated string containing JavaScript code,
/// minifies it, and returns the minified code as a C-compatible string. The caller
/// owns the returned string and is responsible for freeing it with the appropriate
/// FFI deallocation function (e.g. `free_string`, `free_const_string`).
///
/// # Parameters
///
/// * `code` - C string pointer containing the JavaScript code to minify
/// * `error` - Mutable reference to a C char that will contain error message if minification fails
///
/// # Returns
///
/// * On success: Raw pointer to a null-terminated C string containing the minified JavaScript
/// * On failure: Null pointer, with error message populated in the `error` parameter
///
/// # Safety
///
/// This function is unsafe because:
/// * It dereferences raw pointers
/// * It converts between C and Rust string representations
/// * It allocates memory that must be freed by the caller
///
/// # Examples
///
/// ```c
/// const char* js_code = "function hello() { console.log('Hello, world!'); }";
/// char error[256] = {0};
/// char* result = minify_js(js_code, error);
/// if (result) {
///     // Use the minified JavaScript
///     free_string(result); // Free when done
/// } else {
///     // Handle error
///     printf("Error: %s\n", error);
/// }
/// ```
#[no_mangle]
pub extern "C" fn minify_js(code: *const c_char, error: &mut c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(code) }.to_str()
        .expect("failed to convert code to &str");
    let file = File::FileName(FileName::Custom("input.js".into()), input.into());
    match minify(file) {
        Some(output) => CString::new(output)
            .expect("failed to serialize code").into_raw(),
        None => {
            unsafe {
                *error = *CString::new("failed to minify code")
                    .expect("failed to convert error message to CString")
                    .into_raw()
            };
            std::ptr::null_mut()
        }
    }
}

/// Get a JavaScript file
///
/// This function gets a JavaScript file from a file path or a file name with its content.
///
/// # Arguments
///
/// * `file` - The file to get
/// * `cm` - The source map
///
/// # Returns
///
/// The JavaScript file as a source file or an error
///
/// # Errors
///
/// This function returns an error if the file cannot be loaded or created.
fn get_js_file(file: File, cm: Arc<SourceMap>) -> Result<Arc<SourceFile>, Error> {
    Ok(match file {
        File::FilePath(path) => cm.load_file(path)?,
        File::FileName(name, source) => {
            cm.new_source_file(Lrc::new(name), source.into())
        }
    })
}

/// Converts a result to a C string pointer
///
/// This function converts a result to a C string pointer. If the result is an error,
/// the error is printed to stderr.
///
/// # Arguments
///
/// * `result` - The result to convert
///
/// # Returns
///
/// The result as a C string pointer or a null pointer if the result is an error
fn result_to_char_ptr(result: Result<String, Box<dyn std::error::Error>>) -> *mut c_char {
    match result {
        Ok(output) => {
            CString::new(output).expect("failed to convert output to CString").into_raw()
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Transpile TypeScript/TSX to JavaScript
///
/// This function transpiles TypeScript/TSX code to JavaScript.
///
/// # Arguments
///
/// * `file` - The file name
/// * `input` - The TypeScript/TSX code
///
/// # Returns
///
/// The transpiled JavaScript code as a string or a null pointer if the transpilation fails
///
/// # Examples
///
/// ```c
/// #include "swc.h"
///
/// int main() {
///     const char* file_name = "example.ts";
///     const char* ts_code = R"(
///         interface User { name: string }
///         const greet = (user: User) => `Hello ${user.name}!`;
///     )";
///
///     char* js_code = transpile(file_name, ts_code);
///     printf("%s\n", js_code);
///     free_string(js_code);
///
///     return 0;
/// }
/// ```
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure that the pointers are valid.
///
/// # Panics
///
/// This function panics if the conversion from a raw pointer to a string fails.
/// This should never happen in practice.
///
/// # Errors
///
/// This function returns a null pointer if the transpilation fails.
/// The error is printed to stderr.
///
/// # Memory Management
///
/// The caller is responsible for freeing the memory allocated by this
/// function using the `free_string` function.
///
/// # See Also
///
/// * `free_string`
/// * `transpile_file`
/// * `transpile_tsx_to_js`
/// * `File`
#[no_mangle]
pub extern "C" fn transpile(file: *const c_char, input: *const c_char) -> *mut c_char  {
    let file = unsafe { CStr::from_ptr(file) }.to_str().expect("failed to convert file to &str");
    let input = unsafe { CStr::from_ptr(input) }.to_str().expect("failed to convert input to &str");
    let cm: Lrc<SourceMap> = Default::default();
    result_to_char_ptr(
        transpile_tsx_to_js(cm, File::FileName(FileName::Custom(String::from(file)), input.into()))
    )
}

/// Transpile a TypeScript/TSX file to JavaScript
///
/// This function transpiles a TypeScript/TSX file to JavaScript.
///
/// ### Deprecated
/// This function is deprecated. Use the `compile_file` function instead.
///
/// # Arguments
///
/// * `filename` - The file to transpile
///
/// # Returns
///
/// The transpiled JavaScript code as a string or a null pointer if the transpilation fails
///
/// # Examples
///
/// ```c
/// #include "swc.h"
///
/// int main() {
///     const char* file_name = "example.ts";
///     char* js_code = transpile_file(file_name);
///     printf("%s\n", js_code);
///     free_string(js_code);
///
///     return 0;
/// }
/// ```
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure that the pointers are valid.
///
/// # Panics
///
/// This function panics if the conversion from a raw pointer to a string fails.
/// This should never happen in practice.
///
/// # Errors
///
/// This function returns a null pointer if the transpilation fails.
/// The error is printed to stderr.
///
/// # Memory Management
///
/// The caller is responsible for freeing the memory allocated by this
/// function using the `free_string` function.
///
/// # See Also
///
/// * `free_string`
/// * `transpile`
/// * `transpile_tsx_to_js`
/// * `File`
///
/// ***Note: Deprecated***
#[no_mangle]
pub extern "C" fn transpile_file(filename: *const c_char) -> *mut c_char {
    let file = unsafe { CStr::from_ptr(filename) }.to_str()
        .expect("failed to convert filename to &str");
    let cm: Lrc<SourceMap> = Default::default();
    result_to_char_ptr(transpile_tsx_to_js(cm, File::FilePath(Path::new(file))))
}

/// Frees memory allocated by string-returning FFI functions.
///
/// This function properly deallocates memory that was allocated by functions
/// like `minify_js` and `minify_js_file` which return a `*mut c_char`.
/// It safely converts the raw C string pointer back to a Rust CString,
/// which is then automatically dropped at the end of the function scope.
///
/// # Parameters
///
/// * `s` - Mutable pointer to a C string previously returned by an FFI function
///
/// # Safety
///
/// This function is unsafe because:
/// * It converts a raw pointer back to a Rust type
/// * The pointer must have been previously allocated by Rust's `CString::into_raw()`
/// * The pointer must not have been freed already
/// * The pointer must not be used after this call
///
/// # Examples
///
/// ```c
/// char error[256] = {0};
/// char* result = minify_js(js_code, error);
/// if (result) {
///     // Use result...
///     free_string(result); // Must free when done
/// }
/// ```
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        let _ = CString::from_raw(s);
    };
}


/// Frees memory allocated for constant string pointers.
///
/// This function properly deallocates memory pointed to by a constant C string pointer.
/// It is used for freeing strings that were allocated by Rust but returned as read-only
/// pointers to C. The function safely converts the raw C string pointer back to a Rust
/// CString, which is then automatically dropped at the end of the function scope.
///
/// # Parameters
///
/// * `s` - Constant pointer to a C string previously allocated by a Rust FFI function
///
/// # Safety
///
/// This function is unsafe because:
/// * It converts a raw pointer back to a Rust type
/// * It casts a const pointer to a mutable pointer
/// * The pointer must have been previously allocated by Rust's `CString::into_raw()`
/// * The pointer must not have been freed already
/// * The pointer must not be used after this call
///
/// # Examples
///
/// ```c
/// const char* config = get_configuration();
/// // Use config...
/// free_const_string(config); // Must free when done
/// ```
#[no_mangle]
pub extern "C" fn free_const_string(s: *const c_char) {
    unsafe {
        let _ = CString::from_raw(s as *mut c_char);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_transpile() {
        let file = CString::new("test.ts").expect("failed to convert file to CString");
        let input = CString::new("const a: number = 1;").expect("failed to convert input to CString");
        let output = transpile(file.as_ptr(), input.as_ptr());
        assert_eq!(unsafe { CStr::from_ptr(output).to_str().unwrap() }, "const a = 1;\n");
        free_string(output);
    }

    #[test]
    fn test_transpile_ts() {
        let code = r#"
            interface User {
                name: string;
            }

            const greet = (user: User) => `Hello ${user.name}`;

            const user: User = { name: 'Manne' };

            console.log(greet(user));
        "#;
        let result = "const greet = (user)=>`Hello ${user.name}`;\nconst user = {\n    name: 'Manne'\n};\nconsole.log(greet(user));\n";
        let file = CString::new("test.ts").expect("failed to convert file to CString");
        let input = CString::new(code).expect("failed to convert code to CString");
        let output = transpile(file.as_ptr(), input.as_ptr());
        assert_eq!(unsafe { CStr::from_ptr(output).to_str().unwrap() }, result);
        free_string(output);
    }

    #[test]
    fn test_minify_js() {
        let code = r#"
            function hello() {
                console.log('Hello, world!');
            }
        "#;
        let result = "function hello(){console.log('Hello, world!');}\n".to_string();
        let output = minify(File::FileName(FileName::Custom("input.js".parse().unwrap()), code.into()));
        assert_eq!(output, Some(result));
    }
}