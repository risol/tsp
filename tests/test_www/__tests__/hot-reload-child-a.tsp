/**
 * Hot reload child component A - TSP file
 */

import { getSharedValue } from "./shared-dep.ts";

export default Page(async function(ctx) {
  return (
    <html>
      <head><title>Child A</title></head>
      <body><p>Child A - TSP: {getSharedValue()}</p></body>
    </html>
  );
});
