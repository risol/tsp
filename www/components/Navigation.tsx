/**
 * 导航栏组件
 */

export function Navigation() {
  return (
    <nav style={{
      background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
      padding: '16px 0',
      boxShadow: '0 2px 8px rgba(0, 0, 0, 0.1)',
    }}>
      <div className="container" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ color: 'white', fontSize: '20px', fontWeight: 'bold' }}>
          TSP
        </div>

        <ul style={{
          listStyle: 'none',
          display: 'flex',
          gap: '24px',
          margin: 0,
          padding: 0,
        }}>
          <li>
            <a
              href="/"
              style={{
                color: 'white',
                textDecoration: 'none',
                fontWeight: '500',
                transition: 'opacity 0.3s',
              }}
              onMouseEnter={(e) => e.currentTarget.style.opacity = '0.8'}
              onMouseLeave={(e) => e.currentTarget.style.opacity = '1'}
            >
              首页
            </a>
          </li>
          <li>
            <a
              href="/features"
              style={{
                color: 'white',
                textDecoration: 'none',
                fontWeight: '500',
                transition: 'opacity 0.3s',
              }}
              onMouseEnter={(e) => e.currentTarget.style.opacity = '0.8'}
              onMouseLeave={(e) => e.currentTarget.style.opacity = '1'}
            >
              功能特性
            </a>
          </li>
          <li>
            <a
              href="/features/request-info"
              style={{
                color: 'white',
                textDecoration: 'none',
                fontWeight: '500',
                transition: 'opacity 0.3s',
              }}
              onMouseEnter={(e) => e.currentTarget.style.opacity = '0.8'}
              onMouseLeave={(e) => e.currentTarget.style.opacity = '1'}
            >
              请求信息
            </a>
          </li>
          <li>
            <a
              href="/form"
              style={{
                color: 'white',
                textDecoration: 'none',
                fontWeight: '500',
                transition: 'opacity 0.3s',
              }}
              onMouseEnter={(e) => e.currentTarget.style.opacity = '0.8'}
              onMouseLeave={(e) => e.currentTarget.style.opacity = '1'}
            >
              表单示例
            </a>
          </li>
        </ul>
      </div>
    </nav>
  );
}
