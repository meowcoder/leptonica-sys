extern crate bindgen;

use std::env;
use std::path::PathBuf;
#[cfg(windows)]
use vcpkg;

// const MINIMUM_LEPT_VERSION: &str = "1.80.0";

fn compile_leptonica() -> Option<String> {
    let dst = cmake::Config::new("leptonica")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("SW_BUILD", "OFF")
        .define("ENABLE_ZLIB", "OFF")
        .define("ENABLE_PNG", "OFF")
        .define("ENABLE_GIF", "OFF")
        .define("ENABLE_JPEG", "OFF")
        .define("ENABLE_TIFF", "OFF")
        .define("ENABLE_WEBP", "OFF")
        .define("ENABLE_OPENJPEG", "OFF")
        .build();

    let pkg_config_path = dst.join("lib/pkgconfig");
    let _ = std::fs::rename(pkg_config_path.join("lept_Release.pc"), pkg_config_path.join("lept.pc"));
    std::env::set_var("PKG_CONFIG_PATH", &pkg_config_path);

    let pk = pkg_config::Config::new().probe("lept").unwrap();
    let _ = std::fs::hard_link(dst.join(format!("lib/lib{}.a", pk.libs[0])), dst.join("lib/libleptonica.a"));

    println!("cargo:root={}", dst.display());
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=leptonica");

    Some(dst.join("include").display().to_string())
}

#[cfg(windows)]
fn find_leptonica_system_lib() -> Option<String> {
    println!("cargo:rerun-if-env-changed=LEPTONICA_INCLUDE_PATH");
    println!("cargo:rerun-if-env-changed=LEPTONICA_LINK_PATHS");
    println!("cargo:rerun-if-env-changed=LEPTONICA_LINK_LIBS");

    let vcpkg = || {
        let lib = vcpkg::Config::new().find_package("leptonica").unwrap();

        let include = lib
            .include_paths
            .iter()
            .map(|x| x.to_string_lossy())
            .collect::<String>();
        Some(include)
    };

    let include_path = env::var("LEPTONICA_INCLUDE_PATH").ok();
    let link_paths = env::var("LEPTONICA_LINK_PATHS").ok();
    let link_paths = link_paths.as_deref().map(|x| x.split(','));
    let link_libs = env::var("LEPTONICA_LINK_LIBS").ok();
    let link_libs = link_libs.as_deref().map(|x| x.split(','));
    if let (Some(include_path), Some(link_paths), Some(link_libs)) =
        (include_path, link_paths, link_libs)
    {
        for link_path in link_paths {
            println!("cargo:rustc-link-search={}", link_path)
        }

        for link_lib in link_libs {
            println!("cargo:rustc-link-lib={}", link_lib)
        }

        Some(include_path)
    } else {
        vcpkg()
    }
}

// we sometimes need additional search paths, which we get using pkg-config
// we can use leptonica installed anywhere on Linux.
// if you change install path(--prefix) to `configure` script.
// set `export PKG_CONFIG_PATH=/path-to-lib/pkgconfig` before.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
fn find_leptonica_system_lib() -> Option<String> {
    let pk = pkg_config::Config::new().probe("lept").unwrap();
    // Tell cargo to tell rustc to link the system proj shared library.
    println!("cargo:rustc-link-search=native={:?}", pk.link_paths[0]);
    println!("cargo:rustc-link-lib={}", pk.libs[0]);

    let mut include_path = pk.include_paths[0].clone();
    if include_path.ends_with("leptonica") {
        include_path.pop();
    }
    Some(include_path.to_str().unwrap().into())
}

#[cfg(all(
    not(windows),
    not(target_os = "macos"),
    not(target_os = "linux"),
    not(target_os = "freebsd")
))]
fn find_leptonica_system_lib() -> Option<String> {
    println!("cargo:rustc-link-lib=lept");
    None
}

fn main() {
    println!("cargo:rerun-if-env-changed=LEPTONICA_BUNDLE");

    let clang_extra_include = if std::env::var_os("LEPTONICA_BUNDLE").is_some() {
        compile_leptonica()
    } else {
        find_leptonica_system_lib()
    };

    let mut bindings = bindgen::Builder::default().header("wrapper.h");

    if let Some(include_path) = clang_extra_include {
        bindings = bindings.clang_arg(format!("-I{}", include_path));
    }

    let bindings = bindings
        // Remove warnings about improper_ctypes
        .blocklist_function("strtold")
        .blocklist_function("qecvt")
        .blocklist_function("qfcvt")
        .blocklist_function("qgcvt")
        .blocklist_function("qecvt_r")
        .blocklist_function("qfcvt_r")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
