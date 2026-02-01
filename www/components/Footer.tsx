/**
 * 页脚组件
 * 简洁风格
 */
export function Footer() {
  const currentYear = new Date().getFullYear();
  return (
    <footer className="mt-5 py-4 text-center bg-light">
      {/* 品牌区域 */}
      <div className="mb-3">
        <h5 className="text-dark mb-2">TSP - TypeScript Server Page</h5>
        <p className="text-muted small mb-0">
          高性能模板服务器
        </p>
      </div>

      {/* 快速链接 */}
      <div className="d-flex justify-content-center gap-3 mb-3 flex-wrap">
        <a href="/" className="text-decoration-none text-muted small">
          首页
        </a>
        <span className="text-muted">•</span>
        <a href="/demos" className="text-decoration-none text-muted small">
          演示
        </a>
        <span className="text-muted">•</span>
        <a href="/features" className="text-decoration-none text-muted small">
          功能
        </a>
        <span className="text-muted">•</span>
        <a href="/tspinfo" className="text-decoration-none text-muted small">
          服务器信息
        </a>
      </div>

      {/* 版权信息 */}
      <div className="text-muted small">
        <p className="mb-1">
          © {currentYear} TSP. Powered by{" "}
          <a href="https://deno.com" target="_blank" className="text-decoration-none text-reset">
            Deno
          </a>
          {" + "}
          <a href="https://preactjs.com" target="_blank" className="text-decoration-none text-reset">
            Preact
          </a>
        </p>
        <p className="mb-0">
          基于 TypeScript + JSX 构建的现代化 Web 框架
        </p>
      </div>
    </footer>
  );
}

export default Footer;
