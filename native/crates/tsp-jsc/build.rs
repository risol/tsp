fn main() {
    println!("cargo:rerun-if-env-changed=TSP_WEBKIT_ROOT");
    println!("cargo:rerun-if-changed=cxx/tsp_jsc.cpp");
    println!("cargo:rerun-if-changed=include/tsp_jsc.h");

    if std::env::var_os("CARGO_FEATURE_NATIVE_FFI").is_none() {
        return;
    }

    let Some(webkit_root) = std::env::var_os("TSP_WEBKIT_ROOT") else {
        println!("cargo:warning=native-ffi enabled without TSP_WEBKIT_ROOT; using link-only stubs");
        cc::Build::new()
            .cpp(true)
            .file("cxx/tsp_jsc_stub.cpp")
            .include("include")
            .compile("tsp_jsc_bridge_stub");
        return;
    };
    let webkit_root = std::path::PathBuf::from(webkit_root);
    let include = webkit_root.join("include");
    let lib = webkit_root.join("lib");
    if !include.is_dir() || !lib.is_dir() {
        panic!("TSP_WEBKIT_ROOT must contain include and lib directories");
    }
    if cfg!(windows) && !include.join("wtf").join("PlatformEnableWin.h").is_file() {
        panic!(
            "TSP_WEBKIT_ROOT is not a Windows WebKit build: include/wtf/PlatformEnableWin.h is missing"
        );
    }

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("cxx/tsp_jsc.cpp")
        .include(&include)
        .include("include")
        .warnings(true)
        .warnings_into_errors(false);
    let unicode_include = include.join("wtf").join("unicode");
    if unicode_include.is_dir() {
        build.include(unicode_include);
    }
    if cfg!(windows) {
        build.flag_if_supported("/EHsc");
        build.flag_if_supported("/std:c++20");
        // WebKit's portable headers use Clang's pointer-width macro, which
        // MSVC does not provide. The Rust target guarantees this Windows
        // build is 64-bit, so define the header contract explicitly.
        build.flag("/D__SIZEOF_POINTER__=8");
        build.flag("/D__BYTE_ORDER__=1234");
        build.flag("/D__ORDER_LITTLE_ENDIAN__=1234");
        build.flag("/D__ORDER_BIG_ENDIAN__=4321");
    } else {
        build.flag_if_supported("-fexceptions");
        build.flag_if_supported("-std=c++20");
    }
    build.compile("tsp_jsc_bridge");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=JavaScriptCore");
    println!("cargo:rustc-link-lib=static=WTF");
    println!("cargo:rustc-link-lib=static=bmalloc");
    if cfg!(windows) {
        for library in ["sicudt", "sicuin", "sicuuc"] {
            println!("cargo:rustc-link-lib=static={library}");
        }
    } else if cfg!(target_os = "linux") {
        for library in ["icui18n", "icuuc", "icudata"] {
            println!("cargo:rustc-link-lib=static={library}");
        }
    }
}
