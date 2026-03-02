/**
 * Simple .ts import test page
 */

import { appConfig } from "./config.ts";

export default Page(async function(_ctx, { response }) {
  return response.json({
    success: true,
    appName: appConfig.name,
    version: appConfig.version,
  });
});
