export default Page(async function (context, { query, createZod }) {
  const z = await createZod();
  const { method, url } = context;
  const { name = 'World' } = query(z.object({
    name: z.string().default('World')
  }));

  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <title>Home - TSP Test</title>
      </head>
      <body>
        <h1>Home</h1>
        <p>Welcome to TSP!</p>
        <p>Current Path: {url.pathname}</p>
        <p>Request Method: {method}</p>
        <p>URL Parameter: {name}</p>
      </body>
    </html>
  );
});
