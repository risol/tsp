import { Navigation } from "./Navigation.tsx";
import { Footer } from "./Footer.tsx";
import { getCustomStyles } from "./Styles.tsx";
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
        <link href="/static/css/bootstrap.min.css" rel="stylesheet" />
        <style>{getCustomStyles()}</style>
        <script src="/static/js/htmx.min.js" defer></script>
      </head>
      <body className="bg-light">
        <Navigation />
        <div className="container py-4 mt-4">
          {children}
        </div>
        <Footer />
        <script src="/static/js/bootstrap.bundle.min.js"></script>
      </body>
    </html>
  );
}

export default Layout;
