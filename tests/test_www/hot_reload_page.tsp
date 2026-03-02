import { HotReloadWrapper } from "./components/HotReloadWrapper.tsx";

export default async function() {
  return (
    <html>
      <head>
        <title>TSP Hot Reload Test</title>
      </head>
      <body>
        <h1>Hot Reload Test - TSP</h1>
        <HotReloadWrapper />
      </body>
    </html>
  );
}
