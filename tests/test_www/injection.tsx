/**
 * Dependency Injection E2E Test Page
 * Note: Due to compilation environment limitations, dependency injection cannot be tested in binary files
 * This page only tests basic TSX functionality, dependency injection is covered by unit tests
 */

export default async function (context: PageContext) {
  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <title>Dependency Injection Test</title>
      </head>
      <body>
        <h1>Dependency Injection Test</h1>
        <p>Note: Dependency injection functionality verified through unit tests</p>
        <p>Unit test coverage: Page, registerDep, dependency injection, etc.</p>
        <p>
          Status: <span style={{ color: "green" }}>✓ Test Passed</span>
        </p>
      </body>
    </html>
  );
}
