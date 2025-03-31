/// # swc_ffi - FFI bindings for swc
///
/// An C/C++ Interop for the swc Rust library.
use std::{
    ffi::{CStr, CString},
    os::raw::c_char
};
use log::debug;
use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc, FileName, SourceMap
};
use swc_ecma_parser::{
    lexer::Lexer,
    Parser,
    StringInput,
    Syntax
};

#[no_mangle]
// FIXME: https://github.com/swc-project/swc/blob/main/crates/swc_ecma_transforms_typescript/examples/ts_to_js.rs
pub extern "C" fn transpile(file: *const c_char, input: *const c_char) -> *mut c_char  {
    let file = unsafe { CStr::from_ptr(file) }.to_str().expect("failed to convert file to &str");
    let input = unsafe { CStr::from_ptr(input) }.to_str().expect("failed to convert input to &str");
    let cm: Lrc<SourceMap> = Default::default();
    let handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false,
                                  Some(cm.clone()));

    let fm = cm.new_source_file(
        FileName::Custom(file.into()).into(),
        String::from_utf8(input.as_bytes().to_vec())
            .expect("failed to convert file into utf8 string"),
    );
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Es(Default::default()),
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let module = parser
        .parse_module()
        .map_err(|e| {
            // Unrecoverable fatal error occurred
            e.into_diagnostic(&handler).emit()
        })
        .expect("failed to parser module");

    debug!("{:?}", module);
    if module.shebang.is_none() {
        return std::ptr::null_mut()
    }

    match CString::new(module.shebang.expect("Something went wrong").as_str()) {
        Ok(s) => s,
        Err(_) => {
            return std::ptr::null_mut()
        }
    }.into_raw()
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        let _ = CString::from_raw(s);
    };
}

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
    fn test_transpile_js() {
        let file = CString::new("test.ts").expect("failed to convert file to CString");
        let input = CString::new("const a = 1;").expect("failed to convert input to CString");
        let output = transpile(file.as_ptr(), input.as_ptr());
        assert_eq!(unsafe { CStr::from_ptr(output).to_str().unwrap() }, "var a = 1;\n");
        free_string(output);
    }

    #[test]
    fn test_transpile_ts() {
        let file = CString::new("test.ts").expect("failed to convert file to CString");
        //let input = CString::new("interface User { name: string; }\n\nconst greet = (user: User) => `Hello ${user.name}`;\n\nconst user: User = { name: 'Manne' };\nconsole.log(greet(user));\n").expect("failed to convert input to CString");
        let input = CString::new("const a = 1;").expect("failed to convert input to CString");
        let output = transpile(file.as_ptr(), input.as_ptr());
        assert_eq!(unsafe { CStr::from_ptr(output).to_str().unwrap() }, "var a = 1;\n");
        free_string(output);
    }
}