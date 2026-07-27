/**
 * Fragment routing + htmx helper demo - TSX twin for tooling/editor support
 *
 * See fragments_demo.tsp for the canonical version. Both files must
 * stay in sync.
 */

export const fragments: FragmentMap = {
  table: Fragment(async () => {
    return (
      <table id="demo-table" data-source="fragment">
        <thead>
          <tr>
            <th>id</th>
            <th>name</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>1</td>
            <td>apple</td>
          </tr>
          <tr>
            <td>2</td>
            <td>banana</td>
          </tr>
          <tr>
            <td>3</td>
            <td>cherry</td>
          </tr>
        </tbody>
      </table>
    );
  }),

  echo: Fragment(async (_ctx, { query, createZod }) => {
    const z = await createZod();
    const { msg } = query(z.object({ msg: z.string().default("hello") }));
    return <span id="demo-echo" data-msg={msg}>{msg}</span>;
  }),

  json: Fragment(async (_ctx, { query, createZod, response }) => {
    const z = await createZod();
    const { n } = query(z.object({ n: z.coerce.number().default(1) }));
    return response.json({ ok: true, n });
  }),
};

export default Page(async function (ctx) {
  return (
    <html>
      <head>
        <title>Fragments Demo</title>
        <HtmxScript
          defaultSwap="outerHTML"
          timeout={5000}
          historyCacheSize={20}
        />
      </head>
      <body>
        <h1>Fragments Demo</h1>

        <section>
          <h2>Table (auto-fetched initial)</h2>
          <HtmxFragment
            page="/fragments_demo.tsp"
            name="table"
            trigger="every 5s"
          />
        </section>

        <section>
          <h2>Echo (with query)</h2>
          <HtmxFragment
            page="/fragments_demo.tsp"
            name="echo"
            trigger="click from:#echo-btn"
          />
          <button id="echo-btn" type="button">Fetch echo</button>
        </section>

        <section>
          <h2>JSON (Response return)</h2>
          <p>
            Fetched via:
            <code>{hxUrl("/fragments_demo.tsp", "json")}?n=42</code>
          </p>
        </section>
      </body>
    </html>
  );
});
