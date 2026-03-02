/**
 * Test Internal Component Import
 * Verify that files starting with __ can be imported but cannot be accessed directly
 */

import { InternalGreeting, INTERNAL_DATA } from "./__internal_component.tsx";

export default Page(async function(ctx, { response }) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <title>Internal Component Import Test</title>
      </head>
      <body>
        <h1>Internal Component Import Test</h1>

        <section>
          <h2>Component Rendering</h2>
          <InternalGreeting name="World" />
        </section>

        <section>
          <h2>Data Access</h2>
          <pre>{JSON.stringify(INTERNAL_DATA, null, 2)}</pre>
        </section>

        <section>
          <h2>Explanation</h2>
          <p>OK: __internal_component.tsx can be imported</p>
          <p>Error: __internal_component.tsx cannot be accessed directly via HTTP</p>
        </section>
      </body>
    </html>
  );
});
