/**
 * 导航栏组件 - 专业版 Bootstrap 5 响应式导航栏
 * 带有渐变背景、悬停效果和品牌标识
 */

export function Navigation() {
  return (
    <nav className="navbar navbar-expand-lg navbar-dark navbar-gradient sticky-top">
      <div className="container">
        <a className="navbar-brand navbar-brand-custom d-flex align-items-center gap-2" href="/">
          <span className="icon-float">⚡</span>
          <span>TSP</span>
        </a>

        <button
          className="navbar-toggler"
          type="button"
          data-bs-toggle="collapse"
          data-bs-target="#navbarNav"
          aria-controls="navbarNav"
          aria-expanded="false"
          aria-label="Toggle navigation"
        >
          <span className="navbar-toggler-icon"></span>
        </button>

        <div className="collapse navbar-collapse" id="navbarNav">
          <ul className="navbar-nav ms-auto gap-1">
            <li className="nav-item">
              <a className="nav-link nav-link-custom" href="/">
                🏠 首页
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link nav-link-custom" href="/features">
                ✨ 功能特性
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link nav-link-custom" href="/demos">
                🎯 演示
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link nav-link-custom" href="/session_demo">
                🔐 Session
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link nav-link-custom" href="/cookie_demo">
                🍪 Cookies
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link nav-link-custom" href="/features/request-info">
                📡 请求信息
              </a>
            </li>
          </ul>
        </div>
      </div>
    </nav>
  );
}
