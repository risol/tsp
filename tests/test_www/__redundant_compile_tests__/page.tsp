import { getSharedValue } from "./shared.ts";
import { getLibValue } from "./lib.ts";

export default Page(async function(ctx) {
  return (
    <html>
      <body>
        <p>Direct: {getSharedValue()}</p>
        <p>ViaLib: {getLibValue()}</p>
      </body>
    </html>
  );
});