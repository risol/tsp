/**
 * Form Test Page - TSP File
 */

export default async function(ctx) {
  return (
    <html>
      <head>
        <title>TSP Form Test</title>
      </head>
      <body>
        <h1>TSP Form Test</h1>
        <form method="POST" action="/form.tsp">
          <input type="text" name="username" placeholder="Username" />
          <button type="submit">Submit</button>
        </form>
      </body>
    </html>
  );
}
