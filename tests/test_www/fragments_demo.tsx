/**
 * Fragment routing demo - TSX twin for tooling/editor support
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
  const initialTable = await fragments.table(ctx);

  return (
    <html>
      <head>
        <title>Fragments Demo</title>
        <script src="/__static/htmx.js"></script>
      </head>
      <body>
        <h1>Fragments Demo</h1>

        <section>
          <h2>Table fragment (polled every 5s)</h2>
          <div
            id="table-slot"
            hx-get="/fragments_demo.tsp/__fragment/table"
            hx-trigger="every 5s"
            hx-swap="outerHTML"
          >
            {initialTable}
          </div>
        </section>

        <section>
          <h2>Echo fragment (with query)</h2>
          <div
            id="echo-slot"
            hx-get="/fragments_demo.tsp/__fragment/echo?msg=hi+from+htmx"
            hx-trigger="click from:#echo-btn"
            hx-swap="outerHTML"
          >
            <em>click the button to fetch</em>
          </div>
          <button id="echo-btn" type="button">Fetch echo</button>
        </section>

        <section>
          <h2>JSON fragment (Response return)</h2>
          <p>
            Fetched via:
            <code>GET /fragments_demo.tsp/__fragment/json?n=42</code>
          </p>
        </section>
      </body>
    </html>
  );
});
