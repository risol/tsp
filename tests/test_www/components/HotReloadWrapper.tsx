import { HotReloadComponent } from "./HotReloadComponent.tsx";

export function HotReloadWrapper() {
  return <div data-testid="wrapper"><HotReloadComponent /></div>;
}