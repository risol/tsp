/*
 * Compile Bun's pinned mimalloc unity translation unit with the options used
 * by the WebKit SDK. The JSC archive calls mi_* directly, so a compatible
 * implementation must live in the same executable. Keeping it in the bridge
 * makes the allocator dependency explicit without changing Rust's allocator.
 */
#define MI_STATIC_LIB 1
#define MI_SKIP_COLLECT_ON_EXIT 1
#define MI_NO_PROCESS_DETACH 1
#if defined(__linux__)
#define MI_DEFAULT_ALLOW_THP 0
#endif

#include "../../../vendor/mimalloc/src/static.c"
