/// # swc_ffi - FFI bindings for swc
///
/// An C/C++ Interop for the swc Rust library.
use std::sync::Arc;
use std::{
    ffi::{CStr, CString},
    os::raw::c_char
};
use anyhow::Context;
use log::debug;
use swc::{try_with_handler, TransformOutput};
use swc_core::common::GLOBALS;
use swc_core::{
    common::{FileName, SourceMap},
};

/// Output struct to hold the transpiled code, source map, and diagnostics
///
/// ### Fields
///
/// * `output` - the transpiled code
/// * `code` - the transpiled code
/// * `map` - the source map
/// * `diagnostics` - the diagnostics
///
/// ### Safety
///
/// The fields of this struct are pointers to C strings. The caller is responsible for freeing the memory allocated for these strings.
///
/// ### Example
///
/// ```
/// use swc_ts::Output;
/// use std::ffi::CString;
///
/// let output = Output {
///     output: CString::new("output").expect("failed to convert output to CString").into_raw(),
///     code: CString::new("code").expect("failed to convert code to CString").into_raw(),
///     map: CString::new("map").expect("failed to convert map to CString").into_raw(),
///     diagnostics: CString::new("diagnostics").expect("failed to convert diagnostics to CString").into_raw(),
/// };
/// ```
#[repr(C)]
pub struct Output {
    output: *mut c_char,
    code: *mut c_char,
    map: *mut c_char,
    diagnostics: *mut c_char,
}

/// Implementation of the Output struct
impl Output {
    /// Convert a TransformOutput to an Output
    ///
    /// ### Arguments
    ///
    /// * `output` - the TransformOutput to convert
    ///
    /// ### Returns
    ///
    /// The Output struct
    pub fn from_transform_output(output: TransformOutput) -> Self {
        let code = CString::new(output.code).expect("failed to convert code to CString");
        let map = match output.map {
            Some(m) => CString::new(m).expect("failed to convert map to CString"),
            None => CString::new("").expect("failed to convert empty map to CString"),
        };
        let diagnostics = CString::new(output.diagnostics.join("\n")).expect("failed to convert diagnostics to CString");
        let output = match output.output {
            Some(o) => CString::new(o).expect("failed to convert output to CString"),
            None => CString::new("").expect("failed to convert empty output to CString"),
        };
        Self {
            output: output.into_raw(),
            code: code.into_raw(),
            map: map.into_raw(),
            diagnostics: diagnostics.into_raw(),
        }
    }
}

#[no_mangle]
// FIXME: https://github.com/swc-project/swc/blob/main/crates/swc_ecma_transforms_typescript/examples/ts_to_js.rs
pub extern "C" fn transpile_js(file: *const c_char, input: *const c_char) -> Output  {
    let cm = Arc::<SourceMap>::default();
    let compiler = swc::Compiler::new(cm.clone());
    let output = GLOBALS.set(&Default::default(), || {
        try_with_handler(cm.clone(), Default::default(), |handler| {
            let file_name = FileName::Custom(unsafe {
                CStr::from_ptr(file).to_string_lossy().into_owned()
            } );
            let source = unsafe {
                CStr::from_ptr(input).to_string_lossy().into_owned()
            };
            println!("filename: {}", file_name);
            println!("source:\n{}", source);
            let fm = cm.new_source_file_from(Arc::from(file_name), Arc::from(source));
            println!("source file name: {}", fm.name);
            println!("source file source:\n{}", fm.src);
            compiler.process_js_file(fm, handler, &Default::default())
                .context("failed to process js file")
        })
    }).expect("failed to compile ts");
    let o = Output::from_transform_output(output);
    debug!("code: {:?}", unsafe { CStr::from_ptr(o.code).to_str().unwrap() });
    debug!("output: {:?}", unsafe { CStr::from_ptr(o.output).to_str().unwrap() });
    debug!("diagnostics: {:?}", unsafe { CStr::from_ptr(o.diagnostics).to_str().unwrap() });
    debug!("map: {:?}", unsafe { CStr::from_ptr(o.map).to_str().unwrap() });
    o
}

#[no_mangle]
pub extern "C" fn free_output(output: Output) {
    unsafe {
        let _ = CString::from_raw(output.output);
        let _ = CString::from_raw(output.code);
        let _ = CString::from_raw(output.map);
        let _ = CString::from_raw(output.diagnostics);
    };
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        let _ = CString::from_raw(s);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_transpile_js() {
        let file = CString::new("test.ts").expect("failed to convert file to CString");
        let input = CString::new("const a = 1;").expect("failed to convert input to CString");
        let output = transpile_js(file.as_ptr(), input.as_ptr());
        assert_eq!(unsafe { CStr::from_ptr(output.code).to_str().unwrap() }, "var a = 1;\n");
        free_output(output);
    }

    #[test]
    fn test_transpile_ts() {
        let file = CString::new("test.ts").expect("failed to convert file to CString");
        let input = CString::new("interface User { name: string; }\n\nconst greet = (user: User) => `Hello ${user.name}`;\n\nconst user: User = { name: 'Manne' };\nconsole.log(greet(user));\n").expect("failed to convert input to CString");
        let output = transpile_js(file.as_ptr(), input.as_ptr());
        assert_eq!(unsafe { CStr::from_ptr(output.code).to_str().unwrap() }, "var a = 1;\n");
        free_output(output);
    }
}