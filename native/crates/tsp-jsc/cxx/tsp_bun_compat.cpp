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
