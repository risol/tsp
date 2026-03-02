/**
 * Child file C - Shared dependency test
 */

import { getSharedValue } from "./shared-dep.ts";

export default Page(async function () {
  return <div>Child C: {getSharedValue()}</div>;
});
