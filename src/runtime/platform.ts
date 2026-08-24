/**
 * Small Bun/Node compatibility boundary used by the TSP application layer.
 *
 * Runtime primitives live here so the framework does not leak platform APIs into
 * page code. Bun provides the Node-compatible modules used below, while the
 * Bun fork supplies the TSP module loader separately.
 */

import {
  copyFile as copyFileImpl,
  mkdir as mkdirImpl,
  opendir,
  readFile as readFileImpl,
  rename as renameImpl,
  rm as rmImpl,
  stat as statImpl,
  writeFile as writeFileImpl,
} from "node:fs/promises";
import { statSync as statSyncNative } from "node:fs";
import { hostname as hostnameImpl } from "node:os";
import { dirname } from "node:path";

export interface FileInfo {
  size: number;
  mtime: Date | null;
  isDirectory: boolean;
  isFile: boolean;
}

export interface DirEntry {
  name: string;
  isDirectory: boolean;
  isFile: boolean;
  isSymbolicLink: boolean;
}

function toFileInfo(stat: {
  size: number;
  mtime: Date;
  isDirectory(): boolean;
  isFile(): boolean;
}): FileInfo {
  return {
    size: stat.size,
    mtime: stat.mtime,
    isDirectory: stat.isDirectory(),
    isFile: stat.isFile(),
  };
}

export const runtime = {
  args: process.argv.slice(2),
  env: process.env,
  pid: process.pid,
  ppid: process.ppid,
  execPath: process.execPath,
  cwd: () => process.cwd(),
  hostname: () => hostnameImpl(),
  build: {
    os: process.platform,
    arch: process.arch,
    target: `${process.platform}-${process.arch}`,
  },
  exit(code = 0): never {
    process.exit(code);
  },
  async readFile(path: string): Promise<Uint8Array> {
    return new Uint8Array(await readFileImpl(path));
  },
  async readTextFile(path: string): Promise<string> {
    return readFileImpl(path, "utf8");
  },
  async writeFile(
    path: string,
    data: Uint8Array | string,
    options: { append?: boolean } = {},
  ): Promise<void> {
    await writeFileImpl(path, data, options.append ? { flag: "a" } : undefined);
  },
  async writeTextFile(
    path: string,
    data: string,
    options: { append?: boolean } = {},
  ): Promise<void> {
    await writeFileImpl(
      path,
      data,
      options.append ? { encoding: "utf8", flag: "a" } : { encoding: "utf8" },
    );
  },
  async mkdir(path: string, options: { recursive?: boolean } = {}): Promise<void> {
    await mkdirImpl(path, options);
  },
  remove(path: string, options: { recursive?: boolean } = {}): Promise<void> {
    return rmImpl(path, { recursive: options.recursive ?? false, force: false });
  },
  stat(path: string): Promise<FileInfo> {
    return statImpl(path).then(toFileInfo);
  },
  statSync(path: string): FileInfo {
    return toFileInfo(statSyncNative(path));
  },
  copyFile: copyFileImpl,
  rename: renameImpl,
  async *readDir(path: string): AsyncGenerator<DirEntry> {
    const directory = await opendir(path);
    for await (const entry of directory) {
      yield {
        name: entry.name,
        isDirectory: entry.isDirectory(),
        isFile: entry.isFile(),
        isSymbolicLink: entry.isSymbolicLink(),
      };
    }
  },
};

export function isNotFound(error: unknown): boolean {
  return (error as NodeJS.ErrnoException | null)?.code === "ENOENT";
}

export function isAlreadyExists(error: unknown): boolean {
  return (error as NodeJS.ErrnoException | null)?.code === "EEXIST";
}

export { dirname };
