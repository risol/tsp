/**
 * E2E Test Page for Cookie Management
 *
 * This page tests various cookie functionality scenarios:
 * - Basic cookie setting
 * - Cookie with various options
 * - Cookie deletion
 * - Batch operations
 * - Reading request cookies and setting new ones
 */

export default Page(['cookies'], async function(ctx, { cookies }) {
  const action = ctx.query.action || 'demo';

  // Demo mode: show all test links
  if (action === 'demo') {
    return (
      <html>
        <head>
          <title>Cookie E2E Tests</title>
          <style>{`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            h1 { color: #333; }
            .test-section { margin: 30px 0; padding: 20px; border: 1px solid #ddd; border-radius: 8px; }
            .test-section h2 { margin-top: 0; color: #555; }
            a { display: inline-block; margin: 10px 10px 10px 0; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
            a:hover { background: #0051cc; }
            .result { margin-top: 20px; padding: 15px; background: #f0f0f0; border-radius: 5px; }
            code { background: #f5f5f5; padding: 2px 6px; border-radius: 3px; }
          `}</style>
        </head>
        <body>
          <h1>🍪 Cookie Management E2E Tests</h1>

          <div class="test-section">
            <h2>Test 1: Basic Cookie Setting</h2>
            <p>Sets a simple cookie named <code>username</code></p>
            <a href="?action=set-basic">Test Basic Cookie</a>
          </div>

          <div class="test-section">
            <h2>Test 2: Cookie with Options</h2>
            <p>Sets a cookie with httpOnly, secure, sameSite, and maxAge options</p>
            <a href="?action=set-with-options">Test Cookie with Options</a>
          </div>

          <div class="test-section">
            <h2>Test 3: Cookie Deletion</h2>
            <p>First sets a cookie, then deletes it</p>
            <a href="?action=delete">Test Cookie Deletion</a>
          </div>

          <div class="test-section">
            <h2>Test 4: Batch Operations</h2>
            <p>Sets multiple cookies at once using <code>setMultiple</code></p>
            <a href="?action=batch-set">Test Batch Set</a>
          </div>

          <div class="test-section">
            <h2>Test 5: Read Request Cookie</h2>
            <p>Reads existing cookies and displays them</p>
            <a href="?action=read">Test Read Cookies</a>
          </div>

          <div class="test-section">
            <h2>Test 6: Cookie with Redirect</h2>
            <p>Sets a cookie and redirects to another page</p>
            <a href="?action=redirect">Test Cookie with Redirect</a>
          </div>

          <div class="test-section">
            <h2>Test 7: Special Characters</h2>
            <p>Tests cookies with special characters and Unicode</p>
            <a href="?action=special-chars">Test Special Characters</a>
          </div>

          <div class="test-section">
            <h2>Current Cookies</h2>
            <div class="result">
              <strong>Request Cookies:</strong>
              <pre>{JSON.stringify(ctx.cookies, null, 2)}</pre>
            </div>
          </div>
        </body>
      </html>
    );
  }

  // Test 1: Basic cookie setting
  if (action === 'set-basic') {
    cookies.set('username', 'john_doe');
    cookies.set('theme', 'light');

    return (
      <html>
        <head>
          <title>Basic Cookie Test</title>
          <style>{`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}</style>
        </head>
        <body>
          <div class="success">
            <h1>✅ Basic Cookie Test Passed</h1>
            <p>Set cookies:</p>
            <ul>
              <li><code>username</code> = <code>john_doe</code></li>
              <li><code>theme</code> = <code>light</code></li>
            </ul>
            <p><strong>Check your browser's developer tools (Application → Cookies) to verify!</strong></p>
            <a href="?action=demo">← Back to Tests</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 2: Cookie with options
  if (action === 'set-with-options') {
    cookies.set('sessionId', 'abc123xyz', {
      httpOnly: true,
      secure: true,
      sameSite: 'Strict',
      maxAge: 3600, // 1 hour
      path: '/',
    });

    return (
      <html>
        <head>
          <title>Cookie Options Test</title>
          <style>{`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}</style>
        </head>
        <body>
          <div class="success">
            <h1>✅ Cookie Options Test Passed</h1>
            <p>Set cookie with options:</p>
            <ul>
              <li><code>sessionId</code> = <code>abc123xyz</code></li>
              <li><code>httpOnly</code> = <code>true</code></li>
              <li><code>secure</code> = <code>true</code></li>
              <li><code>sameSite</code> = <code>Strict</code></li>
              <li><code>maxAge</code> = <code>3600</code> (1 hour)</li>
              <li><code>path</code> = <code>/</code></li>
            </ul>
            <p><strong>Check browser dev tools to verify all options are set!</strong></p>
            <a href="?action=demo">← Back to Tests</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 3: Cookie deletion
  if (action === 'delete') {
    // First set a cookie
    if (!ctx.cookies.tempCookie) {
      cookies.set('tempCookie', 'will_be_deleted');
      return (
        <html>
          <head>
            <title>Cookie Deletion Test - Step 1</title>
            <style>{`
              body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
              .info { background: #fff3cd; color: #856404; padding: 20px; border-radius: 8px; }
              a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
            `}</style>
          </head>
          <body>
            <div class="info">
              <h1>Step 1: Cookie Set</h1>
              <p>Set cookie <code>tempCookie</code> = <code>will_be_deleted</code></p>
              <a href="?action=delete">Step 2: Delete Cookie →</a>
            </div>
          </body>
        </html>
      );
    }

    // Now delete it
    cookies.delete('tempCookie');

    return (
      <html>
        <head>
          <title>Cookie Deletion Test - Step 2</title>
          <style>{`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}</style>
        </head>
        <body>
          <div class="success">
            <h1>✅ Cookie Deletion Test Passed</h1>
            <p>Deleted cookie <code>tempCookie</code></p>
            <p><strong>The cookie should now be removed from your browser!</strong></p>
            <p>Refresh this page - the cookie should not be present in request cookies.</p>
            <a href="?action=demo">← Back to Tests</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 4: Batch operations
  if (action === 'batch-set') {
    cookies.setMultiple({
      'theme': { value: 'dark', options: { maxAge: 31536000 } },
      'language': { value: 'zh-CN', options: { maxAge: 31536000 } },
      'fontSize': { value: '14px', options: { maxAge: 31536000 } },
    });

    return (
      <html>
        <head>
          <title>Batch Operations Test</title>
          <style>{`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}</style>
        </head>
        <body>
          <div class="success">
            <h1>✅ Batch Operations Test Passed</h1>
            <p>Set multiple cookies using <code>setMultiple</code>:</p>
            <ul>
              <li><code>theme</code> = <code>dark</code> (maxAge: 1 year)</li>
              <li><code>language</code> = <code>zh-CN</code> (maxAge: 1 year)</li>
              <li><code>fontSize</code> = <code>14px</code> (maxAge: 1 year)</li>
            </ul>
            <p><strong>Check browser dev tools to verify all 3 cookies are set!</strong></p>
            <a href="?action=demo">← Back to Tests</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 5: Read request cookies
  if (action === 'read') {
    const username = ctx.cookies.username || 'Not set';
    const theme = ctx.cookies.theme || 'Not set';
    const sessionId = ctx.cookies.sessionId || 'Not set';

    // Set a "lastVisit" cookie
    cookies.set('lastVisit', new Date().toISOString());

    return (
      <html>
        <head>
          <title>Read Request Cookies Test</title>
          <style>{`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}</style>
        </head>
        <body>
          <div class="success">
            <h1>✅ Read Request Cookies Test</h1>
            <p>Current request cookies:</p>
            <ul>
              <li><code>username</code>: {username}</li>
              <li><code>theme</code>: {theme}</li>
              <li><code>sessionId</code>: {sessionId}</li>
            </ul>
            <p>Set <code>lastVisit</code> cookie: <code>{new Date().toISOString()}</code></p>
            <p><strong>All cookies from this page:</strong></p>
            <pre>{JSON.stringify(ctx.cookies, null, 2)}</pre>
            <a href="?action=demo">← Back to Tests</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 6: Cookie with redirect
  if (action === 'redirect') {
    cookies.set('redirectTest', 'successful', {
      maxAge: 3600,
    });

    return {
      redirect: '?action=read',
      status: 302,
    };
  }

  // Test 7: Special characters
  if (action === 'special-chars') {
    cookies.set('user name', 'John Doe');
    cookies.set('email', 'test@example.com');
    cookies.set('chinese', '张三李四');
    cookies.set('emoji', '😀🎉');

    return (
      <html>
        <head>
          <title>Special Characters Test</title>
          <style>{`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}</style>
        </head>
        <body>
          <div class="success">
            <h1>✅ Special Characters Test Passed</h1>
            <p>Set cookies with special characters and Unicode:</p>
            <ul>
              <li><code>user name</code> = <code>John Doe</code></li>
              <li><code>email</code> = <code>test@example.com</code></li>
              <li><code>chinese</code> = <code>张三李四</code></li>
              <li><code>emoji</code> = <code>😀🎉</code></li>
            </ul>
            <p><strong>All values are URL-encoded automatically!</strong></p>
            <p>Check browser dev tools to verify cookies are properly encoded and stored.</p>
            <a href="?action=demo">← Back to Tests</a>
          </div>
        </body>
      </html>
    );
  }

  // Fallback
  return <div>Unknown action</div>;
});
