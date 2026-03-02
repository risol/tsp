import { getB } from "./B.ts";
export default Page(async function(ctx) {
  return <div>{getB()}</div>;
});