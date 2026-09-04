# ADR 0011: Versioned host and JavaScript protocol

Status: Accepted

`RequestEnvelope` and `ResponseEnvelope` are serialized from `tsp-core` and
carry protocol version, request ID, generation, explicit body kind, headers,
effects, and structured errors. HTTP adapters translate at the boundary; they
do not define runtime protocol types.

Request data is passed to a cached JavaScript function as a JSON value through
the JSC adapter. It is never interpolated into JavaScript source code.
