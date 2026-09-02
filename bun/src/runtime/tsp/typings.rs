//! TypeScript declaration files for the `tsp:*` builtin modules.
//!
//! Plan §11 ("Tooling") lists "IDE typings" as one of the
//! five dev-workflow items the contract should ship. The
//! `tspserver typings` subcommand writes the three
//! declaration files emitted by this module into a directory
//! the user adds to their `tsconfig.json` `include` list
//! (typically `.tsp-types/`).
//!
//! The hand-rolled shape mirrors the wrap-prelude's
//! `__tspServer = Object.freeze({...})` in `jsx.rs` (slice
//! 16 + 17 + 18 + 22 + Amendment 1 + Amendment 2 + Amendment
//! 4). If a future slice adds a new name to the wrap, this
//! module MUST grow the matching declaration in the same
//! commit -- the e2e (`tspserver_typings_emits_three_dts_files`)
//! pins the exact strings below so a drift between the
//! runtime and the typings surfaces as a hard test failure.
//!
//! Why hand-rolled and not auto-generated from the wrap?
//! The wrap builds a `const __tspServer = Object.freeze({...})`
//! literal whose entries are bound at module-eval time. The
//! type signature of each entry is scattered across the slice
//! that added it (e.g. `Bun.password` lives in slice 17c, the
//! `util` namespace in slice 18). Auto-extraction would either
//! need a JSC pass at runtime (expensive) or a custom parser
//! for the wrap preamble (brittle). Hand-rolling is ~200
//! lines of `.d.ts` text that the e2e pins by string; future
//! slices touch the runtime AND the typings in one commit.
//!
//! Bun-builtin types (`Bun.password`, `Bun.markdown`, etc.)
//! are intentionally NOT re-exported from `bun-types` here.
//! The user can install `bun-types` and augment the
//! `UtilNamespace` interface themselves if they need
//! the full typing; the slice 18 surface we expose is
//! hand-rolled so the user does not need a devDependency
//! just to satisfy the type-checker.

/// TypeScript declarations for `tsp:server` (plan §16.1).
///
/// Exposes the request/response surface (`Context`,
/// `json` / `redirect` / `text` / `html` / `notFound` /
/// `HttpError`), the fragment builtin (`fragment`, plan
/// §14 / contract item 7), the html-escape helper (`raw`),
/// the id-generation namespace (`nanoid` family, slice
/// 17a), the validation library (`zod`, slice 17b), the
/// database factory (`sql`, slice 17d), the native image pipeline
/// (`Image`), and the bun-builtin `util` namespace (slice 18 + Amendment 2).
pub fn tsp_server_dts() -> &'static str {
    include_str!("../../../../tsp-types/tsp-server.d.ts")
}

/// TypeScript declarations for `tsp:html` (plan §16.2).
///
/// Mirrors the single-name `raw` export the slice 16b
/// wrap re-exposes under `tsp:html`. A page that already
/// imports `raw` from one module does not need to
/// re-import it from the other.
pub fn tsp_html_dts() -> &'static str {
    include_str!("../../../../tsp-types/tsp-html.d.ts")
}

/// TypeScript declarations for `tsp:runtime` (plan §16.3).
///
/// Three reads: `version`, `env` (a get/has wrapper around
/// `Bun.env` -- no `toJSON` to prevent env dumps), and
/// `development` (true when the host was started with
/// `TSP_DEVELOPMENT=1`).
pub fn tsp_runtime_dts() -> &'static str {
    include_str!("../../../../tsp-types/tsp-runtime.d.ts")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the three hand-rolled declaration files are
    /// non-empty and have the right shape. If a future slice
    /// adds a name to the wrap (e.g. a new helper on
    /// `__tspServer`), it must also add a matching
    /// declaration to `tsp-server.d.ts` -- this test catches
    /// the obvious case where a slice ships the runtime
    /// change but forgets the typings commit.
    #[test]
    fn declaration_files_are_non_empty_and_module_scoped() {
        let server = tsp_server_dts();
        let html = tsp_html_dts();
        let runtime = tsp_runtime_dts();
        assert!(server.len() > 100, "tsp-server.d.ts is suspiciously short");
        assert!(html.len() > 50, "tsp-html.d.ts is suspiciously short");
        assert!(runtime.len() > 50, "tsp-runtime.d.ts is suspiciously short");
        assert!(
            server.contains("declare module \"tsp:server\""),
            "tsp-server.d.ts must declare the `tsp:server` module"
        );
        assert!(
            html.contains("declare module \"tsp:html\""),
            "tsp-html.d.ts must declare the `tsp:html` module"
        );
        assert!(
            runtime.contains("declare module \"tsp:runtime\""),
            "tsp-runtime.d.ts must declare the `tsp:runtime` module"
        );
    }

    /// Pin every public name the wrap-prelude's
    /// `__tspServer = Object.freeze({...})` (jsx.rs:829)
    /// exports appears as a `tsp:server` declaration. A
    /// drift here means the runtime added a name and the
    /// typings did not catch up (or vice versa).
    #[test]
    fn tsp_server_declares_every_wrap_prelude_name() {
        let server = tsp_server_dts();
        // Response builders.
        for name in &[
            "export function json",
            "export function text",
            "export function html",
            "export function redirect",
            "export function notFound",
            "export class HttpError",
            "export function fragment",
            "export function raw",
            // ID generation (slice 17a).
            "export function nanoid",
            "export function customAlphabet",
            "export function customRandom",
            "export function random",
            // Validation + database (slice 17b/d).
            "export const zod",
            "export const sql",
            // Native image pipeline.
            "export class Image",
            // Bun builtin namespace (slice 18 + Amendment 2).
            "export const util",
        ] {
            assert!(
                server.contains(name),
                "tsp-server.d.ts is missing `{name}`; got:\n{server}"
            );
        }
    }

    /// Pin the `Context` interface declaration. The shape
    /// here is the contract the application writes against;
    /// the wrap's `__tspContext = { method, path, ... }` plus
    /// the per-request fields the host adds (`url`, `request`,
    /// `signal`, `cookies`, `session`, `services`,
    /// `fragment`) must all be present.
    #[test]
    fn tsp_server_declares_context_shape_per_freeze_item_6() {
        let server = tsp_server_dts();
        assert!(
            server.contains("export type HttpMethod = string;"),
            "Context.method must remain an open string type"
        );
        for field in &[
            "method",
            "url",
            "request",
            "params",
            "query",
            "cookies",
            "session",
            "services",
            "signal",
            "fragment",
        ] {
            assert!(
                server.contains(field),
                "Context declaration is missing `{field}`; got:\n{server}"
            );
        }
    }

    /// Pin the `util` namespace exposes the slice 18 + Amendment
    /// 2 surface. The `password` field is the bun native
    /// (Amendment 2 merged it from the separate `password`
    /// export).
    #[test]
    fn tsp_server_util_namespace_lists_slice_18_surface() {
        let server = tsp_server_dts();
        for name in &[
            "randomUUIDv7",
            "hash",
            "CryptoHasher",
            "Glob",
            "TOML",
            "YAML",
            "markdown",
            "escapeHTML",
            "gzipSync",
            "gunzipSync",
            "file",
            "write",
            "which",
            "peek",
            "deepEquals",
            "deepMatch",
            "nanoseconds",
            "env",
            "password",
        ] {
            assert!(
                server.contains(name),
                "util namespace is missing `{name}`; got:\n{server}"
            );
        }
    }
}
