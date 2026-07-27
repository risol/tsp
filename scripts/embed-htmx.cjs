// Helper: vendor htmx.min.js as a TS string literal.
// Run with: node scripts/embed-htmx.cjs
const fs = require("fs");
const path = require("path");

const SRC = path.join(__dirname, "..", "src", "assets", "htmx.min.js");
const DEST = path.join(__dirname, "..", "src", "assets", "htmx.ts");

const js = fs.readFileSync(SRC, "utf8");
const header = [
  "/* eslint-disable */",
  "/**",
  " * htmx.org v1.9.10 - vendored client library.",
  " * Source: https://unpkg.com/htmx.org@1.9.10/dist/htmx.min.js",
  " * License: BSD-2-Clause (https://htmx.org/license/)",
  " */",
  "",
].join("\n");

const body = header + "export const HTMX_JS = " + JSON.stringify(js) + ";\n";
fs.writeFileSync(DEST, body, "utf8");
console.log("wrote", fs.statSync(DEST).size, "bytes to", DEST);
