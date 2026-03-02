export default Page(async function (context, { body, createZod }) {
  const z = await createZod();
  const { method } = context;

  if (method === "POST") {
    // Try to validate body for POST request
    try {
      const data = body(z.object({
        username: z.string().optional()
      }));
      const username = data.username;

      return (
        <html lang="en">
          <head>
            <meta charset="UTF-8" />
            <title>Form Test - TSP</title>
          </head>
          <body>
            <h1>Form Submitted Successfully</h1>
            <p>Welcome, {username || "Anonymous"}!</p>
            <p>Request Method: {method}</p>
          </body>
        </html>
      );
    } catch {
      // If validation fails, ignore error and show form
    }
  }

  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <title>Form Test - TSP</title>
      </head>
      <body>
        <h1>Form Test</h1>
        <form method="POST">
          <label>
            Username:
            <input type="text" name="username" />
          </label>
          <button type="submit">Submit</button>
        </form>
        <p>Request Method: {method}</p>
      </body>
    </html>
  );
});
