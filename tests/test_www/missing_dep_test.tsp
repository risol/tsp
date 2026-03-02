/**
 * Missing Dependency Fallback Test Page - TSP File
 * Tests that the system can automatically recompile when dependency files in cache are missing
 */

import { HotReloadComponent } from "./components/HotReloadComponent.tsx";

export default Page(async function(ctx) {
  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <title>Missing Dependency Fallback Test</title>
      </head>
      <body>
        <h1>Missing Dependency Fallback Test</h1>
        <div data-testid="parent">
          <HotReloadComponent />
        </div>
      </body>
    </html>
  );
});
