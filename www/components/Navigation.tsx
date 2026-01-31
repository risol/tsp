/**
 * 导航栏组件 - Bootstrap 5响应式导航栏
 */

export function Navigation() {
  return (
    <nav className="navbar navbar-expand-lg navbar-dark bg-primary shadow-sm">
      <div className="container">
        <a className="navbar-brand fw-bold" href="/">
          TSP
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
          <ul className="navbar-nav ms-auto">
            <li className="nav-item">
              <a className="nav-link" href="/">
                首页
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link" href="/features">
                功能特性
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link" href="/demos">
                演示
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link" href="/session_demo">
                Session
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link" href="/cookie_demo">
                Cookies
              </a>
            </li>
            <li className="nav-item">
              <a className="nav-link" href="/features/request-info">
                请求信息
              </a>
            </li>
          </ul>
        </div>
      </div>
    </nav>
  );
}
