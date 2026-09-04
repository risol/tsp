/*
 * The distributed WebKit SDK is built from the Bun WebKit fork and retains
 * two optional C hooks in JavaScriptCore. TSP does not provide Bun runtime
 * behavior, but the hooks must have harmless link-time definitions so the
 * standalone JSC adapter can consume the SDK without importing Bun.
 */
extern "C" void Bun__errorInstance__finalize(...)
{
}

extern "C" void Bun__reportUnhandledError(...)
{
}

// The pinned WebKit archive is built with USE(BUN_EVENT_LOOP). In Bun these
// hooks are exported by the executable's Zig timer adapter; the standalone
// TSP host has no Bun event loop, so provide a conservative inert adapter.
// Returning null makes WebKit's RunLoop timer fall back to its generic path
// where available and keeps the SDK linkable on platforms whose archive does
// not include Bun's executable-side symbols.
extern "C" void* WTFTimer__create(void*)
{
    return nullptr;
}

extern "C" void WTFTimer__update(void*, double, bool)
{
}

extern "C" void WTFTimer__deinit(void*)
{
}

extern "C" void WTFTimer__cancel(void*)
{
}

extern "C" bool WTFTimer__isActive(void*)
{
    return false;
}

extern "C" double WTFTimer__secondsUntilTimer(void*)
{
    return 1.0 / 0.0;
}
