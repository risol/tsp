/**
 * 导航栏组件 - Bootstrap 5 响应式导航栏
 * 严肃简洁风格
 */

export function Navigation() {
  return (
    <nav className="navbar navbar-expand-lg navbar-dark bg-dark sticky-top">
      <div className="container">
        <a className="navbar-brand d-flex align-items-center gap-2" href="/">
          <span>⚡</span>
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
          <ul className="navbar-nav ms-auto">
            <li className="nav-item">
              <a className="nav-link" href="/">首页</a>
            </li>
            <li className="nav-item">
              <a className="nav-link" href="/features">功能特性</a>
            </li>
            <li className="nav-item">
              <a className="nav-link" href="/demos">演示</a>
            </li>
            <li className="nav-item dropdown">
              <a
                className="nav-link dropdown-toggle"
                href="#"
                role="button"
                data-bs-toggle="dropdown"
                aria-expanded="false"
              >
                状态管理
              </a>
              <ul className="dropdown-menu">
                <li>
                  <a className="dropdown-item" href="/session_demo">Session 管理</a>
                </li>
                <li>
                  <a className="dropdown-item" href="/cookie_demo">Cookie 管理</a>
                </li>
              </ul>
            </li>
            <li className="nav-item dropdown">
              <a
                className="nav-link dropdown-toggle"
                href="#"
                role="button"
                data-bs-toggle="dropdown"
                aria-expanded="false"
              >
                数据库
              </a>
              <ul className="dropdown-menu">
                <li>
                  <a className="dropdown-item" href="/mysql-demo">MySQL 演示</a>
                </li>
                <li>
                  <a className="dropdown-item" href="/redis-demo">Redis 演示</a>
                </li>
                <li>
                  <a className="dropdown-item" href="/ldap-demo">LDAP 演示</a>
                </li>
              </ul>
            </li>
            <li className="nav-item dropdown">
              <a
                className="nav-link dropdown-toggle"
                href="#"
                role="button"
                data-bs-toggle="dropdown"
                aria-expanded="false"
              >
                测试工具
              </a>
              <ul className="dropdown-menu">
                <li>
                  <a className="dropdown-item" href="/logger_e2e">Logger 测试</a>
                </li>
                <li>
                  <a className="dropdown-item" href="/logger_rotation_e2e">日志归档测试</a>
                </li>
                <li>
                  <hr className="dropdown-divider" />
                </li>
                <li>
                  <a className="dropdown-item" href="/features/request-info">请求信息</a>
                </li>
              </ul>
            </li>
            <li className="nav-item">
              <a className="nav-link" href="/tspinfo">服务器信息</a>
            </li>
          </ul>
        </div>
      </div>
    </nav>
  );
}
