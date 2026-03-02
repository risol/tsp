import { getLib } from "./lib.ts";
export default Page(async function(ctx) {
  return <div>{getLib()}</div>;
});