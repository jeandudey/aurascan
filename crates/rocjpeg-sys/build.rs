// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use std::env;
use std::path::PathBuf;

fn main() {
    // rocJPEG does not ship a pkg-config file, so locate the installation
    // prefix manually.
    println!("cargo:rerun-if-env-changed=ROCJPEG_PATH");
    println!("cargo:rerun-if-env-changed=ROCM_PATH");
    println!("cargo:rerun-if-env-changed=CONDA_PREFIX");
    let prefix = ["ROCJPEG_PATH", "ROCM_PATH", "CONDA_PREFIX"]
        .iter()
        .find_map(env::var_os)
        .map(PathBuf::from)
        .expect("set ROCJPEG_PATH, ROCM_PATH or CONDA_PREFIX to the rocJPEG prefix");

    println!(
        "cargo:rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    println!("cargo:rustc-link-lib=dylib=rocjpeg");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", prefix.join("include").display()))
        .clang_arg("-D__HIP_PLATFORM_AMD__")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("rocJpeg.*")
        .allowlist_type("(RocJpeg|rocJpeg|RocJPEG).*")
        .allowlist_var("ROCJPEG_.*")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
