extern crate cbindgen;

use std::fs;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Configure cbindgen
    let config = cbindgen::Config {
        language: cbindgen::Language::Cxx,
        cpp_compat: true,
        namespace: Some(String::from("swc")),
        documentation: true,
        ..Default::default()
    };

    // Generate bindings
    match cbindgen::generate_with_config(crate_dir.clone(), config) {
        Ok(bindings) => {
            bindings.write_to_file("swc.h");
            println!("cargo:rerun-if-changed=src/lib.rs");
        },
        Err(e) => {
            eprintln!("cbindgen error(swc.h): {}", e);
            std::process::exit(1);
        }
    }

    // Copy the built library to a convenient location
    let profile = build_target::Profile::current()?;

    let target_dir = Path::new(&crate_dir).join("target");
    let (src_lib, dest_lib) = if profile == build_target::Profile::Release {
        (
            target_dir.join("release").join("libswc_ffi.so"),
            Path::new(&crate_dir).join("libswc.so")
        )
    } else {
        (
            target_dir.join("debug").join("libswc_ffi.so"),
            Path::new(&crate_dir).join("libswc-devel.so")
        )
    };

    // Only try to copy if the source file exists
    if src_lib.exists() {
        if let Err(e) = fs::copy(&src_lib, &dest_lib) {
            eprintln!("Failed to copy {} to {}: {}",
                      src_lib.display(), dest_lib.display(), e);
            return Err(Box::new(e));
        } else {
            println!("Successfully copied {} to {}",
                     src_lib.display(), dest_lib.display());
        }
    } else {
        eprintln!("Source library {} does not exist yet", src_lib.display());
    }

    Ok(())
}