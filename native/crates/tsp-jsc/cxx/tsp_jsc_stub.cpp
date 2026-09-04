#include "../include/tsp_jsc.h"

/*
 * Link-only fallback for contract tests and metadata checks. A real
 * native-ffi build must provide TSP_JSC_SDK_ROOT; this fallback never creates
 * a VM and reports initialization failure instead of silently using another
 * JavaScript runtime.
 */
extern "C" TSP_JSC_EXPORT int32_t tsp_jsc_initialize(void)
{
    return 1;
}

extern "C" TSP_JSC_EXPORT TspJscVm* tsp_jsc_vm_create(void)
{
    return nullptr;
}

extern "C" TSP_JSC_EXPORT void tsp_jsc_vm_destroy(TspJscVm*)
{
}

extern "C" TSP_JSC_EXPORT TspJscResult tsp_jsc_evaluate(
    TspJscVm*,
    TspJscBuffer,
    TspJscBuffer)
{
    return {};
}

extern "C" TSP_JSC_EXPORT TspJscResult tsp_jsc_call_json(
    TspJscVm*,
    TspJscBuffer,
    TspJscBuffer)
{
    return {};
}

extern "C" TSP_JSC_EXPORT int32_t tsp_jsc_drain_microtasks(TspJscVm*)
{
    return 1;
}

extern "C" TSP_JSC_EXPORT void tsp_jsc_buffer_free(TspJscBuffer)
{
}
