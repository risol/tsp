import { mkdtempSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const jscSdkRoot = process.env.TSP_JSC_SDK_ROOT ?? process.env.TSP_WEBKIT_ROOT;
if (!jscSdkRoot) {
  if (process.env.TSP_REQUIRE_E2E === "1") {
    console.error("native application E2E requires TSP_JSC_SDK_ROOT");
    process.exit(1);
  }
  console.log("native application E2E skipped: TSP_JSC_SDK_ROOT is not configured");
  process.exit(0);
}
process.env.TSP_JSC_SDK_ROOT = jscSdkRoot;

const packagedManifest = process.env.TSP_NATIVE_E2E_MANIFEST;
const output = packagedManifest
  ? path.dirname(packagedManifest)
  : mkdtempSync(path.join(os.tmpdir(), "tsp-native-e2e-"));
const applicationRoot = process.env.TSP_NATIVE_E2E_ROOT ?? "native/fixtures/pages";
if (!packagedManifest) {
  const compiler = spawnSync(
    process.platform === "win32" ? "node.exe" : "node",
    ["tools/tspc.mjs", "compile", "--root", applicationRoot, "--out", output],
    { cwd: repository, encoding: "utf8", stdio: "inherit" },
  );
  if (compiler.status !== 0) process.exit(compiler.status ?? 1);
}

const binary = process.env.TSP_NATIVE_E2E_BINARY
  ? path.resolve(process.env.TSP_NATIVE_E2E_BINARY)
  : path.join(
      repository,
      "native",
      "target",
      "debug",
      process.platform === "win32" ? "tsp-cli.exe" : "tsp-cli",
    );
if (!process.env.TSP_NATIVE_E2E_SKIP_BUILD) {
  const build = spawnSync(
    process.platform === "win32" ? "cargo.exe" : "cargo",
    ["build", "--manifest-path", "native/Cargo.toml", "-p", "tsp-cli", "-p", "tsp-worker"],
    { cwd: repository, env: process.env, encoding: "utf8", stdio: "inherit" },
  );
  if (build.status !== 0) process.exit(build.status ?? 1);
}

if (!process.env.TSP_NATIVE_E2E_SKIP_BUILD) {
  const nativeTests = spawnSync(
    process.platform === "win32" ? "cargo.exe" : "cargo",
    [
      "test",
      "--manifest-path",
      "native/Cargo.toml",
      "-p",
      "tsp-jsc",
      "--features",
      "native-ffi",
      "--",
      "--test-threads=1",
    ],
    { cwd: repository, env: process.env, encoding: "utf8", stdio: "inherit" },
  );
  if (nativeTests.status !== 0) process.exit(nativeTests.status ?? 1);
}

const server = spawn(binary, ["--manifest", packagedManifest ?? path.join(output, "manifest.json"), "--listen", "127.0.0.1:0"], {
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
  const request = async (route, { method = "GET", headers, body: requestBody } = {}, expectedStatus = 200) => {
    const response = await fetch(`http://${address}${route}`, { method, headers, body: requestBody });
    const responseBody = await response.text();
    if (response.status !== expectedStatus) throw new Error(`${route}: expected ${expectedStatus}, got ${response.status}: ${responseBody}`);
    return { body: responseBody, headers: response.headers, status: response.status };
  };
  if (applicationRoot === "pages") {
    const root = await request("/");
    if (!root.body.includes("Hello GET") || !root.body.includes("path=/")) throw new Error("application root route mismatch");
    const user = await request("/users/42?q=hello");
    if (!user.body.includes("User 42") || !user.body.includes("path=/users/42")) throw new Error("dynamic route parameter mismatch");
    const created = await request("/", { method: "POST", body: "native" }, 201);
    if (created.body !== "echo:native (signal=pending)") throw new Error(`POST route mismatch: ${created.body}`);
  } else if (applicationRoot === "native/fixtures/pages") {
    const root = await request("/?q=hello");
    const rootValue = JSON.parse(root.body);
    if (rootValue.path !== "/" || rootValue.query !== "hello" || rootValue.method !== "GET") throw new Error("native root route mismatch");
    const created = await request("/", { method: "POST", body: "native" }, 201);
    if (created.body !== "echo:native" || created.headers.get("x-tsp-case") !== "post") throw new Error("native POST route mismatch");
    const user = await request("/users/42?q=hello");
    if (!user.body.includes("User 42") || !user.body.includes("query=hello")) throw new Error("native dynamic route mismatch");
    const wildcard = await request("/docs/a/b");
    if (wildcard.body !== "catch-all:a/b") throw new Error("native catch-all route mismatch");
    const cookie = await request("/cookies");
    if (!cookie.body.includes("seen=none") || !cookie.body.includes("has=true")) throw new Error("native cookie write mismatch");
    const sid = cookie.headers.get("set-cookie")?.match(/sid=([^;]+)/)?.[1];
    if (sid !== "native-session") throw new Error("native Set-Cookie mismatch");
    const cookieAgain = await request("/cookies", { headers: { cookie: `sid=${sid}` } });
    if (!cookieAgain.body.includes("seen=native-session")) throw new Error("native cookie read mismatch");
    const jsonResponse = await request("/json", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ message: "hello" }),
    });
    const jsonValue = JSON.parse(jsonResponse.body);
    if (jsonValue.received !== "hello" || jsonValue.method !== "POST") throw new Error("native JSON route mismatch");
    const asyncResponse = await request("/async", {}, 201);
    if (asyncResponse.body !== "async:/async" || asyncResponse.headers.get("x-tsp") !== "native") throw new Error("native async route mismatch");
    const jsx = await request("/jsx");
    if (!jsx.body.includes("<main><h1>JSX</h1><p>native</p></main>")) throw new Error("native JSX route mismatch");
    const error = await request("/error", {}, 500);
    if (!error.body.includes("internal server error")) throw new Error("native handler error mismatch");
    await request("/users/42", { method: "POST" }, 405);
  } else {
    const root = JSON.parse((await request("/?q=hello")).body);
    if (root.path !== "/" || root.query !== "hello") throw new Error("root route context mismatch");
    const user = JSON.parse((await request("/users/42")).body);
    if (user.id !== "42") throw new Error("dynamic route parameter mismatch");
    const asyncBody = (await request("/async")).body;
    if (asyncBody !== "async:/async") throw new Error(`async route mismatch: ${asyncBody}`);
  }
  await request("/missing", {}, 404);
  console.log("native application E2E passed: routing, request data, response effects, async, errors, 404, and 405");
} finally {
  server.kill();
  if (!packagedManifest) rmSync(output, { recursive: true, force: true });
}
