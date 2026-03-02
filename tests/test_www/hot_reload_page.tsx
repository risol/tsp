/**
 * Hot Reload Test Page
 * Directly reads file content to avoid component import issues in compiled binary
 */

export default Page(async function(ctx) {
  // Directly read HotReloadUtils.ts file to get version
  let version = "UNKNOWN";
  try {
    const utilsPath = "./tests/test_www/components/HotReloadUtils.ts";
    const content = await Deno.readTextFile(utilsPath);
    const match = content.match(/return "([^"]+)"/);
    if (match) {
      version = match[1];
    }
  } catch (error) {
    version = "ERROR";
  }

  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <title>Hot Reload Test</title>
      </head>
      <body>
        <h1>Hot Reload Test Page</h1>
        <p>Current version: <span data-testid="component">{version}</span></p>
      </body>
    </html>
  );
});
