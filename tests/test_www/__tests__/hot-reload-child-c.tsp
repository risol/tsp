/**
 * Hot reload child component C - TSP file
 */

import { getSharedValue } from "./shared-dep.ts";

export default Page(async function(ctx) {
  return (
    <html>
      <head><title>Child C</title></head>
      <body><p>Child C - TSP: {getSharedValue()}</p></body>
    </html>
  );
});
