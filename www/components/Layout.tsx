interface LayoutProps {
  title: string;
  context: PageContext;
  children: unknown;
}

const style = `
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
    padding: 20px;
  }
  .container {
    max-width: 800px;
    margin: 0 auto;
    background: white;
    border-radius: 12px;
    padding: 40px;
    box-shadow: 0 20px 60px rgba(0,0,0,0.3);
  }
  a { color: #667eea; text-decoration: none; }
  a:hover { text-decoration: underline; }
  pre {
    background: #2d3748;
    color: #e2e8f0;
    padding: 15px;
    border-radius: 6px;
    overflow-x: auto;
    font-size: 13px;
    line-height: 1.5;
  }
` as unknown as Record<string, string>;

export function Layout({ title, children }: LayoutProps) {
  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>{title}</title>
        <style>{style}</style>
      </head>
      <body>
        <div class="container">
          {children}
        </div>
      </body>
    </html>
  );
}

// 导出为默认导出，方便直接使用
export default Layout;
