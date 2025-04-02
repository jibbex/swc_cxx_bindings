/// # swc_ffi - FFI bindings for swc
///
/// An C/C++ Interop for the swc Rust library.
use std::{
    ffi::{CStr, CString},
    os::raw::c_char
};
use std::path::Path;
use swc::try_with_handler;
use swc_common::{errors::{ColorConfig, Handler}, sync::Lrc, FileName, Globals, Mark, SourceMap, GLOBALS};
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
use anyhow::Context;

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
    let fm = match filename {
        File::FilePath(path) => cm.load_file(path)?,
        File::FileName(name, source) => {
            cm.new_source_file(Lrc::new(name), source.into())
        }
    };

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

/// Compile a TypeScript/TSX file
///
/// This function compiles a TypeScript/TSX file to JavaScript.
///
/// # Arguments
///
/// * `filepath` - The file path
/// * `error` - The error message
///
/// # Returns
///
/// The compiled JavaScript code as a string or a null pointer if the compilation fails
#[no_mangle]
pub extern "C" fn compile_file(filepath: *const c_char, error: &mut c_char) -> *mut c_char {
    let path = unsafe { CStr::from_ptr(filepath) }.to_str()
        .expect("failed to convert filepath to &str");
    let cm: Lrc<SourceMap> = Default::default();
    let compiler = swc::Compiler::new(cm.clone());
    let output = GLOBALS
        .set(&Default::default(), || {
            try_with_handler(cm.clone(), Default::default(), |handler| {
                let fm = cm
                    .load_file(Path::new(path))
                    .expect("failed to load file");

                compiler.process_js_file(fm, handler, &Default::default())
                    .context("failed to process file")
            })
        });

    match output {
        Ok(output) => {
            CString::new(output.code)
                .expect("failed to serialize code").into_raw()
        },
        Err(e) => {
            unsafe {
                *error = *CString::new(e.to_string())
                    .expect("failed to convert error message to CString")
                    .into_raw()
            };
            std::ptr::null_mut()
        }
    }
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

/// Free a string
///
/// This function frees a string allocated by the `transpile` or `transpile_file` functions.
///
/// # Arguments
///
/// * `s` - The string to free
///
///
/// # Examples
///
/// ```c
/// #include "swc.h"
///
/// int main() {
///     char* s = "Hello, World!";
///     free_string(s);
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
/// # Memory Management
///
/// This function frees the memory allocated by the `transpile` or `transpile_file` functions.
///
/// # See Also
///
/// * `transpile`
/// * `transpile_file`
/// * `transpile_tsx_to_js`
/// * `File`
/// * `free_const_string`
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        let _ = CString::from_raw(s);
    };
}

/// Free a constant string
///
/// This function frees a constant string allocated by the `transpile` or `transpile_file` functions.
///
/// # Arguments
///
/// * `s` - The constant string to free
///
///
/// # Examples
///
/// ```c
/// #include "swc.h"
///
/// int main() {
///     const char* s = "Hello, World!";
///     free_const_string(s);
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
/// # Memory Management
///
/// This function frees the memory allocated by the `transpile` or `transpile_file` functions.
///
/// # See Also
///
/// * `transpile`
/// * `transpile_file`
/// * `transpile_tsx_to_js`
/// * `File`
/// * `free_string`
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
}