// Placeholder -- mysql2 was rejected in favour of bun's native
// `Bun.SQL` (slice 17d). The `mysql` page-side namespace is served
// by `require("bun").SQL` via `__tspServer.sql`; no embed needed.
// This file is left here as a tombstone for the abandoned approach
// (so a future grep for "mysql2" can see the historical context)
// and is not referenced by any `include_str!` in jsx.rs. Delete in
// a follow-up commit once the slice 17d migration settles.
