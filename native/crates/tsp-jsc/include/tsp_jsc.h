#ifndef TSP_JSC_H
#define TSP_JSC_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#define TSP_JSC_EXPORT __declspec(dllexport)
#else
#define TSP_JSC_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct TspJscVm TspJscVm;

typedef struct TspJscBuffer {
    const uint8_t* ptr;
    size_t len;
} TspJscBuffer;

typedef struct TspJscResult {
    uint8_t ok;
    TspJscBuffer value;
    TspJscBuffer error;
} TspJscResult;

/*
 * The native implementation owns every returned buffer. Call
 * tsp_jsc_buffer_free with the same allocation domain that created it.
 */
TSP_JSC_EXPORT int32_t tsp_jsc_initialize(void);
TSP_JSC_EXPORT TspJscVm* tsp_jsc_vm_create(void);
TSP_JSC_EXPORT void tsp_jsc_vm_destroy(TspJscVm* vm);
TSP_JSC_EXPORT TspJscResult tsp_jsc_evaluate(
    TspJscVm* vm,
    TspJscBuffer source,
    TspJscBuffer filename);
TSP_JSC_EXPORT int32_t tsp_jsc_drain_microtasks(TspJscVm* vm);
TSP_JSC_EXPORT void tsp_jsc_buffer_free(TspJscBuffer buffer);

#ifdef __cplusplus
}
#endif

#endif
