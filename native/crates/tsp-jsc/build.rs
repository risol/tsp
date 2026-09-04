fn main() {
    println!("cargo:rerun-if-env-changed=TSP_JSC_SDK_ROOT");
    println!("cargo:rerun-if-changed=cxx/tsp_jsc.cpp");
    println!("cargo:rerun-if-changed=include/tsp_jsc.h");

    if std::env::var_os("CARGO_FEATURE_NATIVE_FFI").is_none() {
        return;
    }

    let Some(webkit_root) = std::env::var_os("TSP_JSC_SDK_ROOT") else {
        println!(
            "cargo:warning=native-ffi enabled without TSP_JSC_SDK_ROOT; using link-only stubs"
        );
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
        panic!("TSP_JSC_SDK_ROOT must contain include and lib directories");
    }
    let metadata = webkit_root.join("metadata.json");
    if metadata.is_file() {
        let metadata = std::fs::read_to_string(&metadata)
            .expect("TSP_JSC_SDK_ROOT metadata.json must be readable");
        if !metadata.contains("\"sdkVersion\": 1") || !metadata.contains("\"runtimeAbi\": 1") {
            panic!("TSP_JSC_SDK_ROOT metadata.json is incompatible with this runtime");
        }
    } else {
        println!(
            "cargo:warning=TSP_JSC_SDK_ROOT has no metadata.json; SDK provenance is unverified"
        );
    }
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("cxx/tsp_jsc.cpp")
        .file("cxx/tsp_bun_compat.cpp")
        .include(&include)
        .include("include")
        .define("STATICALLY_LINKED_WITH_JavaScriptCore", None)
        .define("STATICALLY_LINKED_WITH_WTF", None)
        .define("STATICALLY_LINKED_WITH_bmalloc", None)
        // The standalone bridge links the release WebKit archive. Match its
        // header configuration even when Cargo is compiling a debug profile;
        // otherwise HeapInlines enables debug-only DFG validation references
        // that are absent from the release archive.
        .define("NDEBUG", None)
        .warnings(true)
        .warnings_into_errors(false);
    if std::env::var("PROFILE").as_deref() == Ok("debug") {
        // The SDK headers are shared with optimized WebKit builds and reject
        // a debug (-O0) bridge unless this explicit compatibility mode is set.
        build.define("RELEASE_WITHOUT_OPTIMIZATIONS", None);
    }
    let unicode_include = include.join("wtf").join("unicode");
    if unicode_include.is_dir() {
        build.include(unicode_include);
    }
    if cfg!(windows) {
        build.flag_if_supported("/EHsc");
        build.flag("/DNOMINMAX");
        // Keep the bridge language mode aligned with the WebKit headers used
        // by the SDK. Current WebKit exposes std::to_underlying and Expected
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
        // Match the SDK's native compilation mode to avoid header/ABI drift.
        if cfg!(target_os = "linux") || cfg!(target_os = "freebsd") {
            build.flag_if_supported("-std=gnu++23");
        } else {
            build.flag_if_supported("-std=c++23");
        }
    }
    build.compile("tsp_jsc_bridge");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=JavaScriptCore");
    println!("cargo:rustc-link-lib=static=WTF");
    println!("cargo:rustc-link-lib=static=bmalloc");
    {
        let library = "mimalloc";
        let static_library = if cfg!(windows) {
            lib.join(format!("{library}.lib"))
        } else {
            lib.join(format!("lib{library}.a"))
        };
        if static_library.is_file() {
            println!("cargo:rustc-link-lib=static={library}");
        }
    }
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
