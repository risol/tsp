/**
 * 页脚组件 - 专业版
 * 带有毛玻璃效果、渐变和悬停动画
 */
export function Footer() {
  const currentYear = new Date().getFullYear();
  return (
    <footer className="footer-enhanced mt-5 py-5 text-center rounded-3">
      {/* 品牌区域 */}
      <div className="mb-4 fade-in">
        <div className="d-inline-flex align-items-center gap-2 mb-3">
          <span className="icon-float" style={{ fontSize: '2rem' }}>⚡</span>
          <h3 className="fs-3 mb-0 text-white fw-bold">TSP</h3>
        </div>
        <p className="fs-6 opacity-90">
          TypeScript Server Page - 高性能模板服务器
        </p>
      </div>

      {/* 快速链接 */}
      <div className="d-flex justify-content-center gap-3 mb-4 flex-wrap">
        <a
          href="/"
          className="text-white text-decoration-none opacity-75 hover-lift"
          style={{ transition: 'all 0.3s ease', padding: '0.5rem 1rem', borderRadius: '0.5rem' }}
        >
          🏠 首页
        </a>
        <a
          href="/demos"
          className="text-white text-decoration-none opacity-75 hover-lift"
          style={{ transition: 'all 0.3s ease', padding: '0.5rem 1rem', borderRadius: '0.5rem' }}
        >
          🎯 演示
        </a>
        <a
          href="/features"
          className="text-white text-decoration-none opacity-75 hover-lift"
          style={{ transition: 'all 0.3s ease', padding: '0.5rem 1rem', borderRadius: '0.5rem' }}
        >
          ✨ 功能
        </a>
        <a
          href="/session_demo"
          className="text-white text-decoration-none opacity-75 hover-lift"
          style={{ transition: 'all 0.3s ease', padding: '0.5rem 1rem', borderRadius: '0.5rem' }}
        >
          🔐 Session
        </a>
        <a
          href="/cookie_demo"
          className="text-white text-decoration-none opacity-75 hover-lift"
          style={{ transition: 'all 0.3s ease', padding: '0.5rem 1rem', borderRadius: '0.5rem' }}
        >
          🍪 Cookies
        </a>
      </div>

      {/* 版权信息 */}
      <div className="fs-6 opacity-75">
        <p className="mb-2">
          © {currentYear} TSP. Powered by{" "}
          <a
            href="https://deno.com"
            target="_blank"
            className="text-white text-decoration-underline fw-bold"
            style={{ transition: 'all 0.3s ease' }}
          >
            Deno
          </a>
          {" + "}
          <a
            href="https://preactjs.com"
            target="_blank"
            className="text-white text-decoration-underline fw-bold"
            style={{ transition: 'all 0.3s ease' }}
          >
            Preact
          </a>
        </p>
        <p className="mb-0 small opacity-75">
          💼 基于 TypeScript + JSX 构建的现代化 Web 框架
        </p>
      </div>
    </footer>
  );
}

export default Footer;
