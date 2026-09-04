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
    let mimalloc_source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../bun/vendor/mimalloc/src/static.c");
    let mimalloc_include = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../bun/vendor/mimalloc/include");
    if !include.is_dir() || !lib.is_dir() {
        panic!("TSP_WEBKIT_ROOT must contain include and lib directories");
    }
    if !mimalloc_source.is_file() || !mimalloc_include.is_dir() {
        panic!("the vendored mimalloc source required by WebKit is missing");
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
        // Keep the bridge language mode aligned with the WebKit headers used
        // by Bun. Current WebKit exposes std::to_underlying and Expected
        // through its public headers, which requires C++23 on Windows.
        build.flag_if_supported("/std:c++23preview");
        // WebKit's portable headers use Clang's pointer-width macro, which
        // MSVC does not provide. The Rust target guarantees this Windows
        // build is 64-bit, so define the header contract explicitly.
        build.flag("/D__SIZEOF_POINTER__=8");
        build.flag("/D__BYTE_ORDER__=1234");
        build.flag("/D__ORDER_LITTLE_ENDIAN__=1234");
        build.flag("/D__ORDER_BIG_ENDIAN__=4321");
    } else {
        build.flag_if_supported("-fexceptions");
        // WebKit's Unix headers use GNU extensions and C++23 library types.
        // Match Bun's native compilation mode to avoid header/ABI drift.
        if cfg!(target_os = "linux") || cfg!(target_os = "freebsd") {
            build.flag_if_supported("-std=gnu++23");
        } else {
            build.flag_if_supported("-std=c++23");
        }
    }
    build.compile("tsp_jsc_bridge");

    // Bun's WebKit build uses bmalloc with the mimalloc C API. The Bun
    // executable normally supplies these symbols as part of its own link,
    // but the standalone TSP executable must own this dependency explicitly.
    // Compile the vendored allocator as a separate archive and repeat its
    // link directive after WebKit so archive resolution can satisfy bmalloc.
    let mut mimalloc = cc::Build::new();
    mimalloc
        .file(&mimalloc_source)
        .include(&mimalloc_include)
        .define("MI_STATIC_LIB", None)
        .define("MI_SKIP_COLLECT_ON_EXIT", Some("1"))
        .define("MI_NO_PROCESS_DETACH", Some("1"))
        .define("MI_BUILD_RELEASE", None)
        .warnings(false);
    if !cfg!(windows) {
        mimalloc.flag_if_supported("-fvisibility=hidden");
        mimalloc.flag_if_supported("-Wno-deprecated");
        mimalloc.flag_if_supported("-Wno-static-in-inline");
        mimalloc.flag_if_supported("-ftls-model=initial-exec");
        if cfg!(target_os = "linux") {
            // glibc hides dl_phdr_info and dl_iterate_phdr unless GNU
            // extensions are enabled. Bun's native allocator build enables
            // this feature implicitly; the standalone C compilation must do
            // it explicitly.
            mimalloc.define("_GNU_SOURCE", None);
        }
    } else {
        mimalloc.flag_if_supported("/EHsc");
    }
    mimalloc.compile("tsp_mimalloc");

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
    // See the archive-order note above. This second directive intentionally
    // places mimalloc after the archives that reference its mi_* symbols.
    println!("cargo:rustc-link-lib=static=tsp_mimalloc");
}
