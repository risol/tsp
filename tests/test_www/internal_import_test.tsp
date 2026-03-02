/**
 * Internal Component Import Test - TSP File
 * Test importing components that do not start with __
 */

import { Header } from "./components/Header.tsx";

export default async function(ctx) {
  return (
    <html>
      <head>
        <title>Internal Import Test</title>
      </head>
      <body>
        <h1>Internal Component Import Test - TSP</h1>
        <Header />
      </body>
    </html>
  );
}
