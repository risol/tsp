import { getB } from "./B.ts";
import { getE } from "./E.ts";
export default Page(async function(ctx) {
  return <div>B:{getB()} E:{getE()}</div>;
});