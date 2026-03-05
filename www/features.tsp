import { Sidebar } from "./components/Sidebar.tsx";
import { Footer } from "./components/Footer.tsx";

export default Page(async function(ctx) {
  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Features - TSP</title>
        <style dangerouslySetInnerHTML={{ __html: `
          * { margin: 0; padding: 0; box-sizing: border-box; }
          :root {
            --primary: #3178c6;
            --primary-dark: #235a97;
            --text: #24292e;
            --text-light: #586069;
            --bg: #ffffff;
            --bg-secondary: #f6f8fa;
            --border: #e1e4e8;
            --code-bg: #f6f8fa;
            --sidebar-width: 240px;
          }
          body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: var(--text);
            background: var(--bg);
          }
          .layout { display: flex; min-height: 100vh; }
          .sidebar {
            width: var(--sidebar-width);
            background: var(--bg-secondary);
            border-right: 1px solid var(--border);
            position: fixed;
            height: 100vh;
            overflow-y: auto;
            padding: 20px;
          }
          .main { margin-left: var(--sidebar-width); flex: 1; }
          .header { border-bottom: 1px solid var(--border); padding: 16px 32px; background: var(--bg); }
          .header-content { max-width: 1200px; margin: 0 auto; display: flex; justify-content: space-between; align-items: center; }
          .logo { font-size: 20px; fontWeight: 600; color: var(--primary); text-decoration: none; }
          .nav-links { display: flex; gap: 24px; }
          .nav-links a { color: var(--text); text-decoration: none; font-size: 14px; }
          .nav-links a:hover { color: var(--primary); }
          .content { max-width: 900px; margin: 0 auto; padding: 40px 32px; }
          h1 { font-size: 36px; margin-bottom: 16px; font-weight: 600; }
          h2 { font-size: 24px; margin: 40px 0 16px; padding-bottom: 8px; border-bottom: 1px solid var(--border); }
          h3 { font-size: 18px; margin: 24px 0 12px; }
          p { margin-bottom: 16px; color: var(--text-light); }
          ul, ol { margin: 16px 0; padding-left: 24px; }
          li { margin-bottom: 8px; color: var(--text-light); }
          code { background: var(--code-bg); padding: 2px 6px; border-radius: 4px; font-size: 14px; }
          pre { background: var(--code-bg); padding: 16px; border-radius: 8px; overflow-x: auto; margin: 16px 0; }
          pre code { background: none; padding: 0; }
          .code-block { position: relative; }
          .code-block .lang { position: absolute; top: 8px; right: 12px; font-size: 12px; color: var(--text-light); }
          .feature-section { margin-bottom: 48px; }
          .feature-icon { font-size: 32px; margin-bottom: 12px; }
          .feature-card {
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 24px;
            margin: 16px 0;
          }
          .feature-card h3 { margin-top: 0; }
          .badge { display: inline-block; background: #e8f5e9; color: #2e7d32; padding: 2px 8px; border-radius: 4px; font-size: 12px; margin-left: 8px; }
          .badge.new { background: #e3f2fd; color: #1976d2; }
          footer { background: var(--bg-secondary); border-top: 1px solid var(--border); padding: 24px 32px; text-align: center; color: var(--text-light); font-size: 14px; }
          table { width: 100%; border-collapse: collapse; margin: 16px 0; }
          th, td { border: 1px solid var(--border); padding: 12px; text-align: left; }
          th { background: var(--bg-secondary); }
        `}} />
      </head>
      <body>
        <div class="layout">
          <Sidebar />
          <div class="main">
            <header class="header">
              <div class="header-content">
                <a href="/" class="logo">TSP</a>
                <nav class="nav-links">
                  <a href="/getting-started">Getting Started</a>
                  <a href="/features">Features</a>
                  <a href="/examples">Examples</a>
                </nav>
              </div>
            </header>

            <div class="content">
              <h1>Features</h1>
              <p>TSP is a Deno-based TypeScript full-stack framework with hot reload, type-safe dependency injection, and production deployment support.</p>

              <div class="feature-section">
                <h2><span class="feature-icon">Hot Reload</h2>
                <p>Changes take effect immediately without manual server restart.</p>
                <ul>
                  <li>Supports nested dependencies at any depth</li>
                  <li>Smart caching - only recompiles modified files</li>
                  <li>Cache consistency under concurrent requests</li>
                </ul>
                <div class="feature-card">
                  <h3>How It Works</h3>
                  <p>TSP uses mtime-based caching. When a file changes, it automatically clears the cache and recompiles. All modules depending on that file are recursively cleared.</p>
                </div>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">Type Safety</h2>
                <p>Full TypeScript support, from data validation to database queries.</p>
                <ul>
                  <li>Global type declarations - no manual imports needed</li>
                  <li>Dependency injection provides complete type inference</li>
                  <li>Schema-first data validation</li>
                </ul>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">Database Integration</h2>
                <p>Built-in MySQL, Redis, LDAP support, all with type-safe APIs.</p>

                <h3>MySQL Schema-first API</h3>
                <table>
                  <tr>
                    <th>Method</th>
                    <th>Description</th>
                  </tr>
                  <tr>
                    <td><code>query</code></td>
                    <td>Multi-row query</td>
                  </tr>
                  <tr>
                    <td><code>queryOne</code></td>
                    <td>Single row (throws if 0 or multiple rows)</td>
                  </tr>
                  <tr>
                    <td><code>queryMaybe</code></td>
                    <td>Optional single row (returns null if 0 rows)</td>
                  </tr>
                  <tr>
                    <td><code>scalar</code></td>
                    <td>Single value query</td>
                  </tr>
                  <tr>
                    <td><code>execute</code></td>
                    <td>Write operations (INSERT/UPDATE/DELETE)</td>
                  </tr>
                  <tr>
                    <td><code>tx</code></td>
                    <td>Transaction</td>
                  </tr>
                  <tr>
                    <td><code>queryPage</code></td>
                    <td>Paginated query</td>
                  </tr>
                </table>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">Dependency Injection</h2>
                <p>Type-safe dependency injection system with on-demand loading.</p>
                <div class="code-block">
                  <span class="lang">tsx</span>
                  <pre><code>{`// Use dependencies directly in page function
export default Page(async function(ctx, { session, createMySQL, logger }) {
  // logger - logging
  logger.info('Page loaded');

  // session - session management
  await session.init();
  await session.set('userId', '123');

  // createMySQL - database connection
  const db = await createMySQL({ host: 'localhost', user: 'root', password: '123456', database: 'test' });

  const users = await db.query('SELECT * FROM users');
  return <div>User count: {users.length}</div>;
});`}</code></pre>
                </div>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">Out of the Box</h2>
                <p>Built-in features, no extra installation needed:</p>
                <ul>
                  <li><strong>Session</strong> - Secure and reliable session storage</li>
                  <li><strong>Cookies</strong> - Easy cookie operations</li>
                  <li><strong>File Upload</strong> - Multi-file, large file support</li>
                  <li><strong>Static Files</strong> - HTML, CSS, JS, images, etc.</li>
                  <li><strong>File Manager</strong> <span class="badge new">NEW</span> - Web-based file management</li>
                  <li><strong>Access Log</strong> - Configurable access logging</li>
                </ul>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">Flexible Configuration</h2>
                <p>JSON/JSONC config support. Changes take effect automatically.</p>
                <div class="code-block">
                  <span class="lang">jsonc</span>
                  <pre><code>{`{
  // Basic config
  "root": "./www",
  "port": 9000,

  // Hot reload
  "hotReload": true,

  // Session config
  "session": {
    "secure": false,
    "maxAge": 86400,
    "sameSite": "Strict"
  },

  // Logger config
  "logger": {
    "level": "INFO",
    "file": ".logs/app.log"
  },

  // File manager
  "fileManager": {
    "enabled": true,
    "path": "/__filemanager",
    "password": "your_password"
  }
}`}</code></pre>
                </div>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">Cross-Platform Build</h2>
                <p>Compile to standalone executables for Windows, Linux, and macOS.</p>
                <div class="code-block">
                  <span class="lang">bash</span>
                  <pre><code># Build release package (cross-platform)
sh ./tsp.sh build:tspserver:rel

# Output in release/ directory
cd release
./tspserver --root ./www --port 9000</code></pre>
                </div>
              </div>

              <h2>More</h2>
              <ul>
                <li><a href="/examples" style={{ color: "#3178c6" }}>Examples</a> - More usage scenarios</li>
                <li><a href="https://github.com/risol/tsp" style={{ color: "#3178c6" }}>GitHub</a> - Full documentation</li>
              </ul>
            </div>

            <Footer />
          </div>
        </div>
      </body>
    </html>
  );
});
