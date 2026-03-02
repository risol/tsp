/**
 * Hot reload child component B - TSP file
 */

import { getSharedValue } from "./shared-dep.ts";

export default Page(async function(ctx) {
  return (
    <html>
      <head><title>Child B</title></head>
      <body><p>Child B - TSP: {getSharedValue()}</p></body>
    </html>
  );
});
