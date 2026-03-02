export default async function (context: PageContext) {
  const { method, url, headers } = context;
  const userAgent = headers.get("user-agent") || "Unknown";

  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <title>API Info - TSP</title>
      </head>
      <body>
        <h1>API Info</h1>
        <h2>Request Info</h2>
        <p>
          Request Method: <strong>{method}</strong>
        </p>
        <p>
          Request Path: <strong>{url.pathname}</strong>
        </p>
        <p>
          User-Agent: <strong>{userAgent}</strong>
        </p>

        <h2>Context Data</h2>
        <ul>
          <li>method: {method}</li>
          <li>url: {url.href}</li>
          <li>pathname: {url.pathname}</li>
        </ul>
      </body>
    </html>
  );
}
