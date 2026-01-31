export default Page(async function(ctx, { createMySQL }) {
  try {
    // 获取数据库统计信息
    const db = await createMySQL({
      host: '127.0.0.1',
      port: 3306,
      user: 'test_user',
      password: 'test123456',
      database: 'test_db'
    });

    const [stats] = await db.query<Array<{ total: number }>>('SELECT COUNT(*) as total FROM users');

    return (
      <html>
        <head>
          <title>MySQL 测试中心</title>
          <meta charset="utf-8" />
          <style>{`
            * { box-sizing: border-box; }
            body {
              font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
              line-height: 1.6;
              color: #333;
              max-width: 1400px;
              margin: 0 auto;
              padding: 20px;
              background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
              min-height: 100vh;
            }
            .container {
              background: white;
              border-radius: 20px;
              padding: 40px;
              box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            }
            .header {
              text-align: center;
              margin-bottom: 40px;
              padding-bottom: 30px;
              border-bottom: 3px solid #667eea;
            }
            .header h1 {
              margin: 0 0 10px 0;
              font-size: 3em;
              background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
              -webkit-background-clip: text;
              -webkit-text-fill-color: transparent;
              background-clip: text;
            }
            .header p {
              margin: 0;
              font-size: 1.2em;
              color: #666;
            }
            .stats {
              display: grid;
              grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
              gap: 20px;
              margin-bottom: 40px;
            }
            .stat-card {
              background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
              color: white;
              padding: 25px;
              border-radius: 15px;
              text-align: center;
              box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4);
              transition: transform 0.3s;
            }
            .stat-card:hover { transform: translateY(-5px); }
            .stat-card .value {
              font-size: 3em;
              font-weight: bold;
              margin: 10px 0;
            }
            .stat-card .label {
              font-size: 0.9em;
              opacity: 0.9;
            }
            .section {
              margin-bottom: 40px;
            }
            .section h2 {
              font-size: 2em;
              margin-bottom: 20px;
              color: #333;
              display: flex;
              align-items: center;
            }
            .section h2::before {
              content: '';
              display: inline-block;
              width: 5px;
              height: 40px;
              background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
              margin-right: 15px;
              border-radius: 3px;
            }
            .card-grid {
              display: grid;
              grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
              gap: 25px;
            }
            .card {
              background: #f8f9fa;
              border: 2px solid #e9ecef;
              border-radius: 15px;
              padding: 25px;
              transition: all 0.3s;
              text-decoration: none;
              color: inherit;
              display: block;
            }
            .card:hover {
              border-color: #667eea;
              box-shadow: 0 8px 25px rgba(102, 126, 234, 0.3);
              transform: translateY(-3px);
            }
            .card h3 {
              margin: 0 0 10px 0;
              font-size: 1.4em;
              color: #667eea;
            }
            .card p {
              margin: 0 0 15px 0;
              color: #666;
              line-height: 1.6;
            }
            .card .tags {
              display: flex;
              flex-wrap: wrap;
              gap: 8px;
            }
            .tag {
              background: #e9ecef;
              padding: 4px 12px;
              border-radius: 20px;
              font-size: 0.85em;
              color: #495057;
            }
            .tag.primary { background: #e7f1ff; color: #1971c2; }
            .tag.success { background: #d3f9d8; color: #2f9e44; }
            .tag.warning { background: #fff3bf; color: #f59f00; }
            .tag.danger { background: #ffe3e3; color: #c92a2a; }
            .quick-links {
              background: linear-gradient(135deg, #667eea15 0%, #764ba215 100%);
              border: 2px solid #667eea;
              border-radius: 15px;
              padding: 30px;
              margin-bottom: 40px;
            }
            .quick-links h3 {
              margin: 0 0 20px 0;
              color: #667eea;
            }
            .quick-links a {
              display: inline-block;
              margin: 8px;
              padding: 10px 20px;
              background: white;
              color: #667eea;
              text-decoration: none;
              border-radius: 8px;
              font-weight: 500;
              transition: all 0.3s;
              border: 2px solid transparent;
            }
            .quick-links a:hover {
              background: #667eea;
              color: white;
              border-color: #667eea;
              transform: translateY(-2px);
              box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
            }
            .footer {
              text-align: center;
              margin-top: 60px;
              padding-top: 30px;
              border-top: 2px solid #e9ecef;
              color: #666;
            }
            .footer a {
              color: #667eea;
              text-decoration: none;
              margin: 0 10px;
            }
            .footer a:hover { text-decoration: underline; }
          `}</style>
        </head>
        <body>
          <div className="container">
            <div className="header">
              <h1>🗄️ MySQL 测试中心</h1>
              <p>TSP MySQL 客户端功能演示和测试平台</p>
            </div>

            <div className="stats">
              <div className="stat-card">
                <div className="value">{(stats as any).total}</div>
                <div className="label">总记录数</div>
              </div>
              <div className="stat-card">
                <div className="value">5</div>
                <div className="label">测试页面</div>
              </div>
              <div className="stat-card">
                <div className="value">12</div>
                <div className="label">测试用例</div>
              </div>
              <div className="stat-card">
                <div className="value">100%</div>
                <div className="label">功能覆盖</div>
              </div>
            </div>

            <div className="quick-links">
              <h3>🚀 快速开始</h3>
              <a href="/mysql-demo.tsx">📊 演示仪表板</a>
              <a href="/mysql-advanced.tsx">🔬 高级功能</a>
              <a href="/mysql-performance.tsx">⚡ 性能测试</a>
              <a href="/test-mysql">🔗 简单 API</a>
            </div>

            <div className="section">
              <h2>📚 主要功能页面</h2>
              <div className="card-grid">
                <a href="/mysql-demo.tsx" className="card">
                  <h3>📊 MySQL 演示仪表板</h3>
                  <p>交互式演示仪表板，包含所有基本功能的可视化展示和测试。适合快速了解 MySQL 客户端的功能。</p>
                  <div className="tags">
                    <span className="tag primary">推荐</span>
                    <span className="tag success">交互式</span>
                    <span className="tag">入门</span>
                  </div>
                </a>

                <a href="/mysql-advanced.tsx" className="card">
                  <h3>🔬 高级功能测试</h3>
                  <p>12 种高级功能测试，包括批量操作、复杂查询、事务处理等。适合深入学习 MySQL 客户端的高级特性。</p>
                  <div className="tags">
                    <span className="tag warning">高级</span>
                    <span className="tag">全面</span>
                  </div>
                </a>

                <a href="/mysql-performance.tsx" className="card">
                  <h3>⚡ 性能测试</h3>
                  <p>MySQL 客户端性能基准测试，包括连接速度、查询性能、批量操作等指标。适合性能优化参考。</p>
                  <div className="tags">
                    <span className="tag danger">性能</span>
                    <span className="tag">基准测试</span>
                  </div>
                </a>

                <a href="/test-mysql" className="card">
                  <h3>🔗 JSON API 示例</h3>
                  <p>简单的 RESTful API 示例，展示如何在 TSX 页面中使用 MySQL 客户端返回 JSON 数据。</p>
                  <div className="tags">
                    <span className="tag success">API</span>
                    <span className="tag">简单</span>
                  </div>
                </a>

                <a href="/test-mysql-insert" className="card">
                  <h3>➕ 插入操作测试</h3>
                  <p>专门测试 MySQL 客户端的 insert 功能，返回插入的记录 ID。</p>
                  <div className="tags">
                    <span className="tag">CRUD</span>
                    <span className="tag">创建</span>
                  </div>
                </a>

                <a href="/test-mysql-update" className="card">
                  <h3>✏️ 更新操作测试</h3>
                  <p>专门测试 MySQL 客户端的 update 功能，返回影响的行数。</p>
                  <div className="tags">
                    <span className="tag">CRUD</span>
                    <span className="tag">更新</span>
                  </div>
                </a>

                <a href="/test-mysql-transaction" className="card">
                  <h3>🔄 事务操作测试</h3>
                  <p>测试 MySQL 客户端的事务功能，包括 beginTransaction、commit 和 rollback。</p>
                  <div className="tags">
                    <span className="tag warning">高级</span>
                    <span className="tag">事务</span>
                  </div>
                </a>
              </div>
            </div>

            <div className="section">
              <h2>📖 功能特性</h2>
              <div className="card-grid">
                <div className="card" style={{ cursor: 'default' }}>
                  <h3>🔗 连接管理</h3>
                  <p>支持连接池配置，自动管理连接的创建、复用和释放。提供灵活的连接参数配置。</p>
                  <div className="tags">
                    <span className="tag primary">连接池</span>
                    <span className="tag">自动管理</span>
                  </div>
                </div>

                <div className="card" style={{ cursor: 'default' }}>
                  <h3>📊 数据查询</h3>
                  <p>支持基本查询和参数化查询，防止 SQL 注入。支持 LIMIT、ORDER BY 等子句。</p>
                  <div className="tags">
                    <span className="tag success">安全</span>
                    <span className="tag">防注入</span>
                  </div>
                </div>

                <div className="card" style={{ cursor: 'default' }}>
                  <h3>✏️ 数据操作</h3>
                  <p>完整的 CRUD 操作支持，包括 insert、update、delete。方法简洁易用。</p>
                  <div className="tags">
                    <span className="tag">CRUD</span>
                    <span className="tag">完整</span>
                  </div>
                </div>

                <div className="card" style={{ cursor: 'default' }}>
                  <h3>🔄 事务支持</h3>
                  <p>完整的事务操作支持，包括 beginTransaction、commit、rollback。确保数据一致性。</p>
                  <div className="tags">
                    <span className="tag warning">ACID</span>
                    <span className="tag">一致性</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="section">
              <h2>🎯 测试场景</h2>
              <div className="card-grid">
                <div className="card" style={{ cursor: 'default' }}>
                  <h3>基本查询</h3>
                  <p>SELECT * FROM users</p>
                  <div className="tags">
                    <span className="tag success">简单</span>
                  </div>
                </div>

                <div className="card" style={{ cursor: 'default' }}>
                  <h3>参数化查询</h3>
                  <p>SELECT * FROM users WHERE id = ?</p>
                  <div className="tags">
                    <span className="tag primary">安全</span>
                  </div>
                </div>

                <div className="card" style={{ cursor: 'default' }}>
                  <h3>批量操作</h3>
                  <p>使用事务批量插入 100 条记录</p>
                  <div className="tags">
                    <span className="tag warning">高效</span>
                  </div>
                </div>

                <div className="card" style={{ cursor: 'default' }}>
                  <h3>事务回滚</h3>
                  <p>测试失败时的事务回滚机制</p>
                  <div className="tags">
                    <span className="tag danger">可靠性</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="footer">
              <p>
                <strong>TSP MySQL 客户端</strong> - 类似 PHP 的数据库使用方式
              </p>
              <p style={{ marginTop: '15px' }}>
                <a href="/">返回首页</a>
                <a href="/MYSQL_INTEGRATION.md">查看文档</a>
                <a href="/mysql-demo.tsx">演示仪表板</a>
              </p>
            </div>
          </div>
        </body>
      </html>
    );
  } catch (error) {
    return (
      <html>
        <head>
          <title>数据库连接错误</title>
          <meta charset="utf-8" />
          <style>{`
            body {
              font-family: Arial, sans-serif;
              max-width: 600px;
              margin: 100px auto;
              padding: 20px;
              text-align: center;
            }
            .error {
              background: #ffe3e3;
              border: 2px solid #c92a2a;
              border-radius: 10px;
              padding: 30px;
            }
            h1 { color: #c92a2a; }
            a {
              display: inline-block;
              margin-top: 20px;
              padding: 10px 20px;
              background: #667eea;
              color: white;
              text-decoration: none;
              border-radius: 5px;
            }
          `}</style>
        </head>
        <body>
          <div className="error">
            <h1>❌ 数据库连接失败</h1>
            <p>无法连接到 MySQL 数据库，请确保：</p>
            <ul style={{ textAlign: 'left', display: 'inline-block' }}>
              <li>MySQL Docker 容器正在运行</li>
              <li>数据库连接配置正确</li>
              <li>网络连接正常</li>
            </ul>
            <br />
            <p>运行 <code>.\docker-start.ps1</code> 启动 MySQL</p>
            <a href="/">返回首页</a>
          </div>
        </body>
      </html>
    );
  }
});
