import type { PageContext } from "../../src/cache.ts";
import { Navigation } from "./Navigation.tsx";
import { Footer } from "./Footer.tsx";
import { getGlobalStyles } from "./Styles.tsx";
import type { ComponentChildren } from "preact";

interface LayoutProps {
  title: string;
  description?: string;
  children: ComponentChildren;
}

export function Layout({ title, description, children }: LayoutProps) {
  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>{title}</title>
        {description && <meta name="description" content={description} />}
        <style>{getGlobalStyles()}</style>
      </head>
      <body style={{
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        minHeight: '100vh',
        paddingBottom: '40px',
      }}>
        <Navigation />
        <div className="container" style={{ marginTop: '40px' }}>
          {children}
        </div>
        <Footer />
      </body>
    </html>
  );
}

export default Layout;

