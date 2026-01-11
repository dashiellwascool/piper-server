use std::{env, path::PathBuf};

use cmake::Config;

const PATH: &str = "external/piper/libpiper";

fn main() {
    let mut c = Config::new(PATH);
    c.build();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

   println!(
        "cargo::rustc-link-search={}",
        out_path.join("build").to_str().unwrap()
    );

    println!("cargo::rustc-link-lib=piper");

    let bindings = bindgen::Builder::default()
        .header(out_path.join("include").join("piper.h").to_str().unwrap())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Failed to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("failed to write bindings");
}
