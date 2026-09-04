import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { compileProject, TspCompileError } from "./tspc.mjs";

function fixture(files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tspc-"));
  for (const [name, source] of Object.entries(files)) {
    const fileName = path.join(root, name);
    fs.mkdirSync(path.dirname(fileName), { recursive: true });
    fs.writeFileSync(fileName, source, "utf8");
  }
  return root;
}

test("compiles a TSP route and emits a route manifest", () => {
  const root = fixture({
    "users/[id].tsp": `import { html } from "tsp:server";
export function GET(ctx: { params: Record<string, string> }) {
  return <main>{ctx.params.id}</main>;
}
`,
    "shared.ts": "export const answer: number = 42;\n",
  });
  const out = path.join(root, ".tsp-build");
  const manifest = compileProject({ root, out });

  assert.deepEqual(manifest.routes[0].path, "/users/:id");
  assert.deepEqual(manifest.routes[0].methods, ["GET"]);
  assert.deepEqual(manifest.modules[0].source, "shared.ts");
  assert.match(fs.readFileSync(path.join(out, "users/[id].js"), "utf8"), /__tsp_jsx/);
  assert.equal(fs.existsSync(path.join(out, "manifest.json")), true);
});

test("rejects a default export in a route", () => {
  const root = fixture({ "index.tsp": "export default function Page() { return null; }\n" });
  assert.throws(
    () => compileProject({ root, out: path.join(root, ".tsp-build") }),
    (error) => error instanceof TspCompileError && error.code === "TSP3001",
  );
});

test("rejects imports from another TSP route", () => {
  const root = fixture({
    "index.tsp": "import { GET } from './other.tsp'; export { GET };\n",
  });
  assert.throws(
    () => compileProject({ root, out: path.join(root, ".tsp-build") }),
    (error) => error instanceof TspCompileError && error.code === "TSP1005",
  );
});

test("maps a catch-all route to the TSP wildcard path", () => {
  const root = fixture({
    "files/[...path].tsp": "export function GET() { return 'ok'; }\n",
  });
  const manifest = compileProject({ root, out: path.join(root, ".tsp-build") });
  assert.equal(manifest.routes[0].path, "/files/*");
});
