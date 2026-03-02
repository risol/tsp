import { getB } from "./B.ts";
import { getC } from "./C.ts";
export default Page(async function(ctx) {
  return <div>B:{getB()} C:{getC()}</div>;
});