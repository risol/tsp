/**
 * Precompilation Tool
 * Compile TSX files in www directory to JS files in cache directory
 */

import { cleanCache, compileAll } from "./precompiler_lib.ts";

const command = Deno.args[0];

switch (command) {
  case "clean":
    await cleanCache();
    break;

  case "build":
  default:
    await cleanCache();
    await compileAll();
    break;
}
