# ADR-0004: Model VM roles relative to the current process

> Status: **Accepted (2026-08-30)**
> Scope: Bun/JSC VM initialization used by TSP embedded worker processes
> Related investigation: `docs/v2/bugs/0003-windows-ci-worker-sigsegv-invalid-handle.md`

## Context

The TSP master launches child processes and calls them workers. That process
role is not the same concept as Bun's in-process `WebWorker`. Each TSP child
starts on its operating-system entry thread, owns the only JSC VM in that
process, and has no `WorkerMessagingProxy` parent.

The previous code represented VM identity with three independent fields:
`is_main_thread`, `worker_ptr`, and `context_id`. TSP selected
`is_main_thread: false` while leaving the other fields empty. The defaults then
selected the auxiliary/macro context sentinel. Rust code still treated the VM
as main-thread in places because `vm.worker` was empty, while C++ treated its
script execution context as non-main. The same VM therefore had contradictory
identities across the boundary.

## Decision

VM initialization uses one `VmRole` value:

- `ProcessMain` publishes the VM as the current process's main VM, uses script
  execution context id 1, and has no worker messaging proxy.
- `Auxiliary` represents macro/debugger VMs, uses the generated-context
  sentinel, and has no worker messaging proxy.
- `WebWorker` carries a non-null `WorkerMessagingProxy` and a concrete context
  id greater than 1 as one unit.

The TSP worker process uses `ProcessMain`. The word "worker" in TSP lifecycle
and protocol code does not imply `VmRole::WebWorker`.

## Rules

1. Determine VM role relative to the process that owns the JSC VM.
2. A child process that owns its only VM uses `ProcessMain`.
3. Use `WebWorker` only when Bun's in-process worker owner and context id both
   exist.
4. Do not derive main-thread behavior independently from `worker_ptr`, context
   id, or `vm.worker`; derive initialization identity from `VmRole`.
5. Keep startup tracing environment-gated and split Rust VM setup from C++ JSC
   VM, client-data, and global-object creation.

## Consequences

- Invalid partial WebWorker configurations fail at VM initialization.
- TSP no longer enters the macro/auxiliary global-object path accidentally.
- Windows CI diagnostics identify the exact native VM initialization boundary
  without changing normal output.
