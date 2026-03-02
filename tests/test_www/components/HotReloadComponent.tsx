import { getVersion } from "./HotReloadUtils.ts";

export function HotReloadComponent() {
  return <div data-testid="component">{getVersion()}</div>;
}