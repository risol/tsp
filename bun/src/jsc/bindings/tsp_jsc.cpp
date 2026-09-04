/*
 * Bun build compatibility wrapper for TSP's standalone JSC bridge.
 *
 * The implementation lives in the TSP native workspace so the standalone
 * runtime and Bun's packaging build use exactly the same ABI implementation.
 */
#define TSP_JSC_BUN_BUILD 1
#include "../../../../native/crates/tsp-jsc/cxx/tsp_jsc.cpp"
