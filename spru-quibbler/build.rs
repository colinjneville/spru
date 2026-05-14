use std::{env, path};

fn main() {
    println!("cargo::rerun-if-changed=scripts/*");

    let mut src = path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    src.push("assets");
    src.push("scripts");

    let build_type = env::var("PROFILE").unwrap();
    copy_to_output::copy_to_output_path(&src, &build_type)
        .expect("Failed to copy scripts folder");
    
}