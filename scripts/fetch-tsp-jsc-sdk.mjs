import { createHash } from "node:crypto";
import { copyFileSync, createReadStream, createWriteStream, existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const specificationPath = path.join(repository, "docs", "reference", "dependencies", "tsp-jsc-sdk.json");
const specification = JSON.parse(readFileSync(specificationPath, "utf8"));

function targetKey() {
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "win32" && process.arch === "x64") return "windows-x64";
  if (process.platform === "darwin" && process.arch === "arm64") return "macos-arm64";
  throw new Error(`unsupported TSP JSC SDK target: ${process.platform}-${process.arch}`);
}

function argument(name) {
  const index = process.argv.indexOf(name);
  return index < 0 ? undefined : process.argv[index + 1];
}

const output = argument("--output");
if (!output) throw new Error("usage: node scripts/fetch-tsp-jsc-sdk.mjs --output DIRECTORY");
const target = targetKey();
const asset = specification.assets[target];
const outputRoot = path.resolve(output);
const temporaryRoot = fsTempDirectory(target);
const archive = path.join(temporaryRoot, asset.name);
const staging = path.join(temporaryRoot, "extracted");
mkdirSync(staging, { recursive: true });

const url = `${specification.repository}/releases/download/${specification.releaseTag}/${asset.name}`;
console.log(`Downloading TSP JSC SDK ${specification.releaseTag} for ${target}`);
const response = await fetch(url);
if (!response.ok || !response.body) throw new Error(`SDK download failed with HTTP ${response.status}`);
await pipeline(Readable.fromWeb(response.body), createWriteStream(archive));

const digest = await sha256(archive);
if (digest !== asset.sha256) {
  throw new Error(`SDK checksum mismatch: expected ${asset.sha256}, got ${digest}`);
}

const extraction = spawnSync("tar", ["-xzf", archive, "-C", staging], { encoding: "utf8" });
if (extraction.status !== 0) throw new Error(`SDK extraction failed: ${extraction.stderr || extraction.stdout}`);
const sdkRoot = findSdkRoot(staging);
if (!sdkRoot) throw new Error("SDK archive does not contain include/JavaScriptCore and lib");

mkdirSync(outputRoot, { recursive: true });
copyDirectory(path.join(sdkRoot, "include"), path.join(outputRoot, "include"));
copyDirectory(path.join(sdkRoot, "lib"), path.join(outputRoot, "lib"));
if (existsSync(path.join(sdkRoot, "licenses"))) copyDirectory(path.join(sdkRoot, "licenses"), path.join(outputRoot, "licenses"));
writeFileSync(path.join(outputRoot, "metadata.json"), `${JSON.stringify({ ...specification, target }, null, 2)}\n`);
console.log(`TSP JSC SDK ready at ${outputRoot}`);

function fsTempDirectory(name) {
  const directory = path.join(os.tmpdir(), `tsp-jsc-sdk-${name}`);
  mkdirSync(directory, { recursive: true });
  return directory;
}

async function sha256(file) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest("hex");
}

function findSdkRoot(directory, depth = 0) {
  if (existsSync(path.join(directory, "include", "JavaScriptCore", "JavaScript.h")) && existsSync(path.join(directory, "lib"))) return directory;
  if (depth >= 3) return undefined;
  for (const entry of readdirSync(directory)) {
    const child = path.join(directory, entry);
    if (statSync(child).isDirectory()) {
      const result = findSdkRoot(child, depth + 1);
      if (result) return result;
    }
  }
  return undefined;
}

function copyDirectory(source, destination) {
  mkdirSync(destination, { recursive: true });
  for (const entry of readdirSync(source, { withFileTypes: true })) {
    const sourcePath = path.join(source, entry.name);
    const destinationPath = path.join(destination, entry.name);
    if (entry.isDirectory()) copyDirectory(sourcePath, destinationPath);
    else copyFileSync(sourcePath, destinationPath);
  }
}
