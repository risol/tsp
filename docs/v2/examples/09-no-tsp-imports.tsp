// 09-no-tsp-imports.tsp -- the wrong way to import another page.
// This file is INTENTIONALLY INVALID; it is kept here as a
// reference for the host's TSP2003 error.
//
// Demonstrates: freeze 2 -- .tsp modules cannot be imported.
// The host refuses to start with this file present (or, if
// already started, refuses to publish any generation that
// references it).

import UsersPage from "./users.tsp"; // <- TSP2003

export const config = { auth: "none" } satisfies PageConfig;

export function GET() {
  return <UsersPage />;
}

interface PageConfig { auth: "none" | "required"; }
