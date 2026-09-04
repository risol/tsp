import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const webkitRoot = process.env.TSP_WEBKIT_ROOT;
if (!webkitRoot) {
  console.log("native application E2E skipped: TSP_WEBKIT_ROOT is not configured");
  process.exit(0);
}

const output = mkdtempSync(path.join(os.tmpdir(), "tsp-native-e2e-"));
const compiler = spawnSync(
  process.platform === "win32" ? "node.exe" : "node",
  ["tools/tspc.mjs", "compile", "--root", "native/fixtures/pages", "--out", output],
  { cwd: repository, encoding: "utf8", stdio: "inherit" },
);
if (compiler.status !== 0) process.exit(compiler.status ?? 1);

const binary = path.join(
  repository,
  "native",
  "target",
  "debug",
  process.platform === "win32" ? "tsp-cli.exe" : "tsp-cli",
);
const build = spawnSync(
  process.platform === "win32" ? "cargo.exe" : "cargo",
  ["build", "--manifest-path", "native/Cargo.toml", "-p", "tsp-cli"],
  { cwd: repository, env: process.env, encoding: "utf8", stdio: "inherit" },
);
if (build.status !== 0) process.exit(build.status ?? 1);

const server = spawn(binary, ["--manifest", path.join(output, "manifest.json"), "--listen", "127.0.0.1:0"], {
  cwd: repository,
  env: process.env,
  stdio: ["ignore", "pipe", "inherit"],
});
const address = await new Promise((resolve, reject) => {
  let text = "";
  server.stdout.on("data", (chunk) => {
    text += chunk;
    const match = text.match(/TSP_LISTENING ([^\r\n]+)/);
    if (match) resolve(match[1]);
  });
  server.once("error", reject);
  server.once("exit", (code) => reject(new Error(`tsp-cli exited before listening (${code})`)));
});

try {
  const get = async (route, expectedStatus = 200) => {
    const response = await fetch(`http://${address}${route}`);
    const body = await response.text();
    if (response.status !== expectedStatus) throw new Error(`${route}: expected ${expectedStatus}, got ${response.status}: ${body}`);
    return body;
  };
  const root = JSON.parse(await get("/?q=hello"));
  if (root.path !== "/" || root.query !== "hello") throw new Error("root route context mismatch");
  const user = JSON.parse(await get("/users/42"));
  if (user.id !== "42") throw new Error("dynamic route parameter mismatch");
  const asyncBody = await get("/async");
  if (asyncBody !== "async:/async") throw new Error(`async route mismatch: ${asyncBody}`);
  await get("/missing", 404);
  console.log("native application E2E passed: root, dynamic, async, and 404 routes");
} finally {
  server.kill();
  rmSync(output, { recursive: true, force: true });
}
