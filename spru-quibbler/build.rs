use std::{env, path};

fn main() {
    println!("cargo::rerun-if-changed=rhai/*");

    let build_type = env::var("PROFILE").unwrap();

    {
        let mut src = path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        src.push("assets");
        src.push("rhai");

        copy_to_output::copy_to_output_path(&src, &build_type)
            .expect("Failed to copy rhai folder");
    }
}