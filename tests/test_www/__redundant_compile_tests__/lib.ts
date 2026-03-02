import { getSharedValue } from "./shared.ts";

export function getLibValue(): string {
  return "LIB_" + getSharedValue();
}