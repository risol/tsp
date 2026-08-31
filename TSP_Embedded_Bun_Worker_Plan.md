# TSP Persistent Isolated Embedded Bun Worker Architecture embedded-worker

# Complete Codex Implementation Plan embedded-worker

## Architecture Clarification

This revision selects the embedded-runtime model:

-   `tspserver` is the Rust master process.
-   The master creates persistent Rust worker child processes.
-   Each worker links and owns an embedded Bun runtime.
-   A worker MUST NOT start Bun as a grandchild process.
-   A request MUST NOT create a process or initialize a new Bun runtime.

## Document Purpose

This document is the master engineering plan for migrating TSP from the
current per-request Bun subprocess execution model to a PHP-FPM-like
Persistent Isolated Bun Worker architecture.

This document is designed for Codex Agent execution.

Codex should execute tasks sequentially:

    TASK-001
    TASK-002
    TASK-003
    ...

Each task must keep the project buildable and tested.

------------------------------------------------------------------------

# 1. Final Architecture

## Target

                     Client

                       |

                 tsp-server

                       |

                Worker Manager

                       |

           +-----------+-----------+

           |           |           |

        Worker Process      Worker Process      Worker Process
        Rust + embedded Bun Rust + embedded Bun Rust + embedded Bun
             |                    |                    |
        TSP Runtime          TSP Runtime          TSP Runtime
             |                    |                    |
        Application           Application           Application

------------------------------------------------------------------------

# 2. Core Rules

## Rule 1

Master process MUST NOT execute user TSP code.

## Rule 2

Bun runtime is linked into and owned by the worker process. The worker is
not a Rust launcher for an external Bun child process.

## Rule 3

Worker processes are disposable.

The system must tolerate:

-   crash
-   memory leak
-   infinite loop
-   native addon failure

## Rule 4

No request may create a new Bun process.

## Rule 5

No worker may create a Bun grandchild process. Bun initialization happens
once inside the worker process and remains alive for the worker lifetime.

## Rule 6

The TSP worker build MUST include the Bun runtime integration. The build
must define the Bun source revision, build flags, platform targets, license
requirements, and any local patches. Using a separately installed `bun`
executable is not the target architecture.

------------------------------------------------------------------------

# 3. Current Architecture Migration

Current:

    Request

     |

    TSP Master

     |

    spawn Bun

     |

    execute

     |

    exit

Target:

    Request

     |

    TSP Master

     |

    Worker Manager

     |

    Persistent Rust Worker

     |

    Embedded Bun Runtime

     |

    execute

------------------------------------------------------------------------

# 4. Repository Migration Strategy

Do not rewrite TSP runtime behavior.

Preserve:

-   Page API
-   Dependency Injection
-   React JSX support
-   Hot reload
-   Static files
-   Sessions
-   Database APIs

Only change execution ownership and process/runtime integration. Existing
TSP request semantics remain unchanged.

------------------------------------------------------------------------

# 5. New Modules

Create:

    src/worker/

     protocol
     client
     manager
     scheduler
     lifecycle


    src/worker-runtime/

     worker-main
     runtime-host
     embedded-bun
     executor


    build/bun-runtime/

     source revision
     platform build integration
     patches
     license metadata


    src/sandbox/

     limits
     cgroup
     namespace

------------------------------------------------------------------------

# 6. Phase 0 - Repository Analysis

## TASK-001

Goal:

Understand current execution flow.

Inspect:

    src/main.ts
    src/router.ts
    src/context.ts
    src/response.ts
    src/runtime/*
    bun/*

Create:

    docs/current-runtime-analysis.md

Document:

-   server startup
-   request flow
-   Bun initialization
-   module loading
-   hot reload

Acceptance:

No code changes.

------------------------------------------------------------------------

# 7. Phase 1 - Extract Execution Layer

## TASK-010

Create:

    src/executor.ts

Move:

-   TSP page loading
-   request execution
-   response creation

Define:

``` ts
interface Executor {

 execute(request): Promise<Response>

}
```

Master should call Executor abstraction.

------------------------------------------------------------------------

# 8. Phase 2 - Worker Protocol

## TASK-020

Create:

    src/worker/protocol.ts

Protocol:

``` ts
enum WorkerMessage {

 Hello,

 Ready,

 Execute,

 Response,

 Cancel,

 Shutdown,

 Heartbeat

}
```

Request:

``` ts
interface ExecuteRequest {

 id:number

 application:string

 method:string

 path:string

 headers:Array<[string,string]>

 body:Uint8Array

}
```

Response:

``` ts
interface ExecuteResponse {

 id:number

 status:number

 headers:Array<[string,string]>

 body:Uint8Array

}
```

Requirements:

-   binary encoding
-   versioning
-   validation

------------------------------------------------------------------------

# 9. Phase 3 - Worker Process

## TASK-030

Create the Rust worker executable with Bun linked into the worker process.

The worker executable MUST NOT launch `bun`, `bun.exe`, or any other Bun
child process. Bun initialization must happen through the embedded runtime
integration defined by TASK-040.

Startup:

    process start

     |

    initialize embedded Bun runtime

     |

    initialize runtime

     |

    connect master

     |

    READY

Worker loop:

    receive request

    execute

    return response

Acceptance:

-   worker starts without spawning a Bun child process
-   Bun runtime is initialized once per worker lifetime
-   worker can execute multiple requests without reinitialization
-   process-tree inspection confirms no Bun grandchild process

------------------------------------------------------------------------

# 10. Phase 4 - Bun Runtime Migration

## TASK-040

Implement Bun embedding and build integration.

Before:

    master

     |

    external Bun subprocess

After:

    Rust worker process

     |

    linked embedded Bun runtime

Move:

-   builtin registration
-   module loader
-   runtime initialization
-   dependency injection setup

Also define:

-   Bun source revision and reproducible build inputs
-   worker-to-Bun runtime initialization boundary
-   native build/link configuration for Windows, Linux, and macOS
-   Bun license and third-party dependency metadata
-   any required local Bun patches

Feasibility gate:

-   identify the Bun runtime entry point and embedding boundary
-   prove that the worker can initialize JavaScriptCore/Bun once
-   prove that TSP modules can be loaded and invalidated from the embedded runtime
-   stop and document the blocker if the selected Bun revision has no viable embedding path

Master keeps:

-   HTTP
-   routing
-   scheduling

Master MUST NOT link or initialize Bun. Only worker binaries contain the
Bun runtime.

------------------------------------------------------------------------

# 11. Phase 5 - Worker Manager

## TASK-050

Implement:

    WorkerManager

Functions:

``` ts
startWorker()

stopWorker()

restartWorker()

selectWorker()

    healthCheck()

WorkerManager starts the compiled Rust worker executable. It MUST NOT
resolve or invoke a separately installed Bun executable.
```

------------------------------------------------------------------------

# 12. Phase 6 - Worker Pool

## TASK-060

Implement:

    WorkerPool

Features:

-   multiple workers
-   load balancing
-   queueing
-   backpressure

Initial scheduling:

Least active requests.

------------------------------------------------------------------------

# 13. Phase 7 - Application Isolation

## TASK-070

Introduce:

    Application

     |

    WorkerGroup

     |

    Worker[]

Each application:

-   independent embedded Bun runtime
-   independent memory
-   independent restart

------------------------------------------------------------------------

# 14. Phase 8 - Timeout System

## TASK-080

Every request requires:

    deadline

Flow:

    timeout

     |

    cancel

     |

    grace period

     |

    kill worker

     |

    restart

------------------------------------------------------------------------

# 15. Phase 9 - Resource Isolation

## TASK-090

Linux:

Implement:

    cgroup v2

    memory.max

    cpu.max

    pids.max

Optional:

    namespace

    seccomp

------------------------------------------------------------------------

# 16. Phase 10 - Worker Recycling

## TASK-100

Recycle worker by:

    max requests

    max memory

    max age

Example:

    10000 requests

    1GB memory

    2 hours

------------------------------------------------------------------------

# 17. Phase 11 - Hot Reload

## TASK-110

Zero downtime reload:

    start new worker

    load application

    health check

    switch traffic

    drain old worker

    shutdown

The replacement worker initializes its own embedded Bun runtime before it
receives traffic. Reload must not depend on starting a Bun grandchild
process.

------------------------------------------------------------------------

# 18. IPC Design

Initial:

    Unix Domain Socket

Later:

    shared memory

Do not optimize before correctness.

------------------------------------------------------------------------

# 19. Error Handling

Worker errors:

    worker crash

     |

    master detects disconnect

     |

    remove worker

     |

    create replacement

Never:

    worker crash

     |

    master crash

------------------------------------------------------------------------

# 20. Testing Plan

## Unit

-   protocol
-   scheduler
-   lifecycle

## Integration

-   worker startup
-   embedded Bun initialization
-   request execution
-   crash recovery
-   timeout
-   no Bun grandchild process
-   repeated requests use the same embedded runtime

## Stress

-   high concurrency
-   memory exhaustion
-   CPU exhaustion
-   worker restart

------------------------------------------------------------------------

# 21. Benchmark Plan

Compare:

Old:

    spawn and initialize Bun per request

New:

    persistent Rust worker with embedded Bun

Measure:

-   startup latency
-   p50 latency
-   p99 latency
-   throughput
-   memory
-   recovery time
-   worker process count and child-process tree
-   runtime initialization count per worker

------------------------------------------------------------------------

# 22. Codex Execution Rules

For every task:

1.  Read existing code first.
2.  Modify minimum files.
3.  Run tests.
4.  Keep build passing.
5.  Do not delete old code before migration success.
6.  Update documentation.

------------------------------------------------------------------------

# 23. Completion Criteria

Migration is complete when:

-   Master never executes user code.
-   Bun runtime is linked into worker processes only.
-   No request spawns or initializes a new Bun process/runtime.
-   No worker creates a Bun child or grandchild process.
-   Worker binaries are reproducibly built with the declared Bun source revision.
-   Worker crash does not affect master.
-   Multiple applications run isolated.
-   Resource limits work.
-   Hot reload works.
-   Performance exceeds subprocess architecture.
