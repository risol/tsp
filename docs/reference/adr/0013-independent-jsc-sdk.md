# ADR 0013: Independent JavaScriptCore SDK

Status: Accepted

The native adapter consumes a target-specific SDK through
`TSP_JSC_SDK_ROOT`. The SDK owns the JavaScriptCore, WTF, bmalloc, ICU, and
allocator ABI. Cargo supplies the allocator binding independently; the TSP
build never reads a vendored runtime source tree to assemble the SDK.

The SDK metadata must be versioned with its target triple, WebKit revision,
allocator, and runtime ABI before it is used for a release build.
