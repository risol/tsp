import { getChild } from "./child.tsx";
export function getLib(): string { return "lib_" + getChild(); }