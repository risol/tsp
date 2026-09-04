# ADR 0010: Native runtime ownership boundaries

Status: Accepted

## Decision

TSP is split into a host, an engine-neutral JavaScript capability boundary, a
JavaScriptCore adapter, and an isolated worker process. The dependency graph is

```text
tsp-cli -> tsp-runtime -> tsp-js <- tsp-jsc -> TSP JSC SDK
                |
             tsp-core <- tsp-http
```

The host owns HTTP lifecycle, routing, generations, admission, worker
lifecycle, and durable state. JavaScriptCore owns only VM and JavaScript
execution. No JSC value, pointer, or allocator-owned buffer crosses a worker
process boundary.

## Consequences

Request execution uses a versioned core envelope and cached JavaScript dispatch
function calls. A worker crash can be detected and replaced without sharing VM
state with the host. The production process worker and the thread worker test
backend use the same route execution contract.
