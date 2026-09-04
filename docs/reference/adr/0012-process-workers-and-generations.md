# ADR 0012: Process workers and last-known-good generations

Status: Accepted

Production workers are OS processes. IPC uses a bounded length-prefixed frame
with a versioned JSON command or event payload. Native pointers and JavaScript
objects are forbidden in IPC.

Application generations are immutable. A candidate is validated and loaded
into workers before the host publishes it. Requests pin the current generation
ID, and a failed candidate leaves the last-known-good generation active.
