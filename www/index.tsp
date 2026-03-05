import { Nav } from "./components/Nav.tsx";
import { Sidebar } from "./components/Sidebar.tsx";
import { Footer } from "./components/Footer.tsx";

export default Page(async function(ctx) {
  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>TSP - TypeScript Full-Stack Framework</title>
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
          .logo { font-size: 20px; font-weight: 600; color: var(--primary); text-decoration: none; }
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
          .feature-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 24px; margin: 24px 0; }
          .feature-card {
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 20px;
            transition: box-shadow 0.2s;
          }
          .feature-card:hover { box-shadow: 0 4px 12px rgba(0,0,0,0.1); }
          .feature-card h3 { margin-top: 0; color: var(--text); }
          .feature-card p { margin-bottom: 0; font-size: 14px; }
          .badge { display: inline-block; background: #e8f5e9; color: #2e7d32; padding: 2px 8px; border-radius: 4px; font-size: 12px; margin-left: 8px; }
          .btn { display: inline-block; background: var(--primary); color: white; padding: 10px 20px; border-radius: 6px; text-decoration: none; font-size: 14px; }
          .btn:hover { background: var(--primary-dark); }
          .btn-secondary { background: var(--bg-secondary); color: var(--text); border: 1px solid var(--border); }
          .btn-secondary:hover { background: var(--border); }
          .cta { text-align: center; padding: 40px 0; }
          .cta .btn { margin: 0 8px; }
          .code-block { position: relative; }
          .code-block .lang { position: absolute; top: 8px; right: 12px; font-size: 12px; color: var(--text-light); }
          footer { background: var(--bg-secondary); border-top: 1px solid var(--border); padding: 24px 32px; text-align: center; color: var(--text-light); font-size: 14px; }
          .highlight { background: #fff3cd; padding: 2px 4px; border-radius: 2px; }
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
              <h1>TSP: TypeScript Full-Stack Framework</h1>
              <p style={{ fontSize: "18px" }}>
                A Deno-based TypeScript full-stack framework. Write .tsp/.tsx files as routes to build modern web applications.
              </p>

              <div class="cta">
                <a href="/getting-started" class="btn">Get Started</a>
                <a href="/features" class="btn btn-secondary">Learn More</a>
              </div>

              <h2>Why TSP?</h2>

              <div class="feature-grid">
                <div class="feature-card">
                  <h3>Simple</h3>
                  <p>Execute .tsp files directly like PHP. No complex build config needed. Write and run.</p>
                </div>
                <div class="feature-card">
                  <h3>Hot Reload</h3>
                  <p>Changes take effect immediately. Supports nested dependencies at any depth.</p>
                </div>
                <div class="feature-card">
                  <h3>Type Safe</h3>
                  <p>Full TypeScript support with complete type inference through dependency injection.</p>
                </div>
                <div class="feature-card">
                  <h3>Database</h3>
                  <p>Schema-first MySQL/Redis/LDAP support with type-safe database query APIs.</p>
                </div>
                <div class="feature-card">
                  <h3>Full-Featured</h3>
                  <p>Built-in Session, Cookies, File Upload, Static Files, and more. Ready to use.</p>
                </div>
                <div class="feature-card">
                  <h3>Flexible Config</h3>
                  <p>JSON/JSONC config support. Changes take effect automatically without server restart.</p>
                </div>
              </div>

              <h2>Installation</h2>

              <p>Use the built-in deno-tsp runtime (no official Deno required):</p>

              <div class="code-block">
                <span class="lang">bash</span>
                <pre><code># Build deno-tsp
sh ./tsp.sh build:denort
sh ./tsp.sh build:deno

# Start dev server
sh ./tsp.sh dev</code></pre>
              </div>

              <p>Common commands:</p>

              <div class="code-block">
                <span class="lang">bash</span>
                <pre><code># Production mode
sh ./tsp.sh start

# Build binary
sh ./tsp.sh build:tspserver
sh ./tsp.sh build:tspserver:rel</code></pre>
              </div>

              <h2>Create Your First Page</h2>

              <p>Create <code>www/hello.tsp</code> in the www directory:</p>

              <div class="code-block">
                <span class="lang">tsx</span>
                <pre><code>{`export default Page(async function(ctx, { logger }) {
  logger.info('Home page visited');

  return (
    <html>
      <head>
        <title>My TSP Website</title>
      </head>
      <body>
        <h1>Welcome to TSP!</h1>
        <p>Current time: {new Date().toLocaleString()}</p>
      </body>
    </html>
  );
});`}</code></pre>
              </div>

              <p>Then visit <code>http://localhost:9000/hello</code> to see your page.</p>

              <h2>Next Steps</h2>

              <ul>
                <li><a href="/getting-started">Getting Started</a> - Complete installation and setup guide</li>
                <li><a href="/features">Features</a> - Learn more about TSP features</li>
                <li><a href="/examples">Examples</a> - Code examples for common scenarios</li>
              </ul>
            </div>

            <Footer />
          </div>
        </div>
      </body>
    </html>
  );
});
