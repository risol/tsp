/*
 * TSP's minimal JavaScriptCore bridge.
 *
 * This file intentionally uses JavaScriptCore's public C API plus the small
 * VM accessor needed to drain microtasks. It does not create an application
 * global object and does not depend on a host runtime, event loop, module
 * loader, or transpiler.
 */
#include "../include/tsp_jsc.h"
#include <JavaScriptCore/APICast.h>
#include <JavaScriptCore/GetVM.h>
#include <JavaScriptCore/HeapInlines.h>
#include <JavaScriptCore/InitializeThreading.h>
#include <JavaScriptCore/JSLock.h>
#include <JavaScriptCore/JavaScript.h>
#include <JavaScriptCore/VM.h>

#include <cstdlib>
#include <cstdio>
#include <cstring>
#include <limits>
#include <mutex>

struct TspJscVm {
    JSGlobalContextRef context;
};

static bool traceEnabled()
{
    static const bool enabled = std::getenv("TSP_JSC_TRACE") != nullptr;
    return enabled;
}

static void trace(const char* stage)
{
    if (!traceEnabled())
        return;
    std::fprintf(stderr, "TSP_JSC_TRACE %s\n", stage);
    std::fflush(stderr);
}

static TspJscBuffer copyString(JSStringRef value)
{
    if (!value)
        return {};
    size_t capacity = JSStringGetMaximumUTF8CStringSize(value);
    auto* bytes = static_cast<unsigned char*>(std::malloc(capacity));
    if (!bytes)
        return {};
    size_t lengthWithTerminator = JSStringGetUTF8CString(
        value,
        reinterpret_cast<char*>(bytes),
        capacity);
    if (lengthWithTerminator == 0) {
        std::free(bytes);
        return {};
    }
    return { bytes, lengthWithTerminator - 1 };
}

static TspJscBuffer copyLiteral(const char* value)
{
    JSStringRef string = JSStringCreateWithUTF8CString(value);
    if (!string)
        return {};
    TspJscBuffer result = copyString(string);
    JSStringRelease(string);
    return result;
}

static TspJscBuffer copyValueAsString(JSContextRef context, JSValueRef value)
{
    JSValueRef conversionException = nullptr;
    JSStringRef string = JSValueToStringCopy(context, value, &conversionException);
    if (!string || conversionException) {
        if (string)
            JSStringRelease(string);
        return copyLiteral("JavaScript value could not be converted to a string");
    }
    TspJscBuffer result = copyString(string);
    JSStringRelease(string);
    return result;
}

static JSStringRef createString(TspJscBuffer input)
{
    if ((!input.ptr && input.len != 0)
        || input.len == std::numeric_limits<size_t>::max()
        || (input.len != 0 && std::memchr(input.ptr, 0, input.len)))
        return nullptr;
    auto* bytes = static_cast<char*>(std::malloc(input.len + 1));
    if (!bytes)
        return nullptr;
    if (input.len)
        std::memcpy(bytes, input.ptr, input.len);
    bytes[input.len] = 0;
    JSStringRef result = JSStringCreateWithUTF8CString(bytes);
    std::free(bytes);
    return result;
}

extern "C" TSP_JSC_EXPORT int32_t tsp_jsc_initialize(void)
{
    trace("initialize.begin");
    // Standalone embedders must perform low-level initialization before
    // creating a VM. JSGlobalContextCreate is not a
    // replacement for initializing WTF's main-thread state and JSC's option
    // table; omitting this can corrupt the VM when Promise microtasks are
    // drained or when the context is released.
    static std::once_flag initializeOnce;
    std::call_once(initializeOnce, [] {
        trace("initialize.main-thread");
        WTF::initializeMainThread();
        trace("initialize.wtf-ready");
        JSC::initialize([] {});
        trace("initialize.jsc-ready");
    });
    trace("initialize.done");
    return 0;
}

extern "C" TSP_JSC_EXPORT TspJscVm* tsp_jsc_vm_create(void)
{
    trace("vm-create.begin");
    auto* vm = static_cast<TspJscVm*>(std::calloc(1, sizeof(TspJscVm)));
    if (!vm)
        return nullptr;
    trace("vm-create.storage-ready");
    vm->context = JSGlobalContextCreate(nullptr);
    if (!vm->context) {
        std::free(vm);
        return nullptr;
    }
    trace("vm-create.context-ready");

    // The public context factory creates the VM and global object, but the
    // embedder still has to publish heap access before using JSC's C++ API.
    // Acquire heap access first,
    // then take the API lock. Skipping it makes JSLockHolder touch an
    // unavailable heap state on Linux and can crash before the lock is held.
    auto* globalObject = toJSGlobalObject(vm->context);
    auto& jscVm = JSC::getVM(globalObject);
    jscVm.heap.acquireAccess();
    trace("vm-create.heap-ready");
    return vm;
}

extern "C" TSP_JSC_EXPORT void tsp_jsc_vm_destroy(TspJscVm* vm)
{
    if (!vm)
        return;
    trace("vm-destroy.begin");
    if (vm->context)
        JSGlobalContextRelease(vm->context);
    trace("vm-destroy.context-released");
    std::free(vm);
    trace("vm-destroy.done");
}

extern "C" TSP_JSC_EXPORT TspJscResult tsp_jsc_evaluate(
    TspJscVm* vm,
    TspJscBuffer source,
    TspJscBuffer filename)
{
    TspJscResult result {};
    trace("evaluate.begin");
    if (!vm || !vm->context
        || (!source.ptr && source.len != 0)
        || (!filename.ptr && filename.len != 0)) {
        result.error = copyLiteral("invalid JSC evaluation input");
        return result;
    }

    JSStringRef sourceString = createString(source);
    JSStringRef filenameString = filename.ptr ? createString(filename) : nullptr;
    trace("evaluate.strings-ready");
    if (!sourceString || (filename.ptr && !filenameString)) {
        if (sourceString)
            JSStringRelease(sourceString);
        if (filenameString)
            JSStringRelease(filenameString);
        result.error = copyLiteral("failed to allocate JSC source string");
        return result;
    }

    JSValueRef exception = nullptr;
    JSValueRef value = JSEvaluateScript(
        vm->context,
        sourceString,
        nullptr,
        filenameString,
        1,
        &exception);
    trace("evaluate.script-done");
    if (exception) {
        result.error = copyValueAsString(vm->context, exception);
    } else if (value) {
        result.ok = 1;
        result.value = copyValueAsString(vm->context, value);
    }

    JSStringRelease(sourceString);
    if (filenameString)
        JSStringRelease(filenameString);
    trace("evaluate.done");
    return result;
}

extern "C" TSP_JSC_EXPORT TspJscResult tsp_jsc_call_json(
    TspJscVm* vm,
    TspJscBuffer function,
    TspJscBuffer argumentJson)
{
    TspJscResult result {};
    trace("call-json.begin");
    if (!vm || !vm->context
        || (!function.ptr && function.len != 0)
        || (!argumentJson.ptr && argumentJson.len != 0)) {
        result.error = copyLiteral("invalid JSC call input");
        return result;
    }

    JSStringRef functionString = createString(function);
    JSStringRef argumentString = createString(argumentJson);
    if (!functionString || !argumentString) {
        if (functionString)
            JSStringRelease(functionString);
        if (argumentString)
            JSStringRelease(argumentString);
        result.error = copyLiteral("failed to allocate JSC call strings");
        return result;
    }

    auto* globalObject = toJSGlobalObject(vm->context);
    auto& jscVm = JSC::getVM(globalObject);
    JSC::JSLockHolder lock(jscVm);
    JSValueRef exception = nullptr;
    JSValueRef functionValue = JSObjectGetProperty(
        vm->context,
        reinterpret_cast<JSObjectRef>(globalObject),
        functionString,
        &exception);
    JSValueRef argumentValue = nullptr;
    if (!exception)
        argumentValue = JSValueMakeFromJSONString(vm->context, argumentString);
    if (!functionValue || exception || !JSValueIsObject(vm->context, functionValue)) {
        result.error = exception
            ? copyValueAsString(vm->context, exception)
            : copyLiteral("cached JSC function was not found");
    } else if (!argumentValue) {
        result.error = copyLiteral("JSC call argument is not valid JSON");
    } else {
        JSValueRef value = JSObjectCallAsFunction(
            vm->context,
            JSValueToObject(vm->context, functionValue, &exception),
            reinterpret_cast<JSObjectRef>(globalObject),
            1,
            &argumentValue,
            &exception);
        if (exception)
            result.error = copyValueAsString(vm->context, exception);
        else if (value) {
            result.ok = 1;
            result.value = copyValueAsString(vm->context, value);
        }
    }
    JSStringRelease(functionString);
    JSStringRelease(argumentString);
    trace("call-json.done");
    return result;
}

extern "C" TSP_JSC_EXPORT int32_t tsp_jsc_drain_microtasks(TspJscVm* vm)
{
    if (!vm || !vm->context)
        return 1;
    trace("microtasks.begin");
    auto* globalObject = toJSGlobalObject(vm->context);
    trace("microtasks.global-ready");
    auto& jscVm = JSC::getVM(globalObject);
    trace("microtasks.vm-ready");
    JSC::JSLockHolder lock(jscVm);
    trace("microtasks.locked");
    jscVm.drainMicrotasks();
    trace("microtasks.done");
    return 0;
}

extern "C" TSP_JSC_EXPORT void tsp_jsc_buffer_free(TspJscBuffer buffer)
{
    // Every buffer returned by this file is malloc-owned. Never pass it to
    // Rust, mimalloc, WebKit, or a C++ delete expression for deallocation.
    std::free(const_cast<unsigned char*>(buffer.ptr));
}
