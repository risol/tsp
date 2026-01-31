/**
 * 预编译工具
 * 将 www 目录中的 TSX 文件编译为缓存目录中的 JS 文件
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
