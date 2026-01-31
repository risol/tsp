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
          <meta name="viewport" content="width=device-width, initial-scale=1" />
          <link href="/static/css/bootstrap.min.css" rel="stylesheet" />
        </head>
        <body className="container py-4">
          <div className="card shadow-lg">
            <div className="card-body p-5">
              <div className="text-center mb-5 pb-4 border-bottom">
                <h1 className="display-5 mb-2">🗄️ MySQL 测试中心</h1>
                <p className="lead text-muted">TSP MySQL 客户端功能演示和测试平台</p>
              </div>

              <div className="row g-4 mb-5">
                <div className="col-md-3">
                  <div className="card bg-primary text-white h-100">
                    <div className="card-body text-center">
                      <div className="display-4 fw-bold">{(stats as any).total}</div>
                      <div className="small opacity-75">总记录数</div>
                    </div>
                  </div>
                </div>
                <div className="col-md-3">
                  <div className="card bg-primary text-white h-100">
                    <div className="card-body text-center">
                      <div className="display-4 fw-bold">5</div>
                      <div className="small opacity-75">测试页面</div>
                    </div>
                  </div>
                </div>
                <div className="col-md-3">
                  <div className="card bg-primary text-white h-100">
                    <div className="card-body text-center">
                      <div className="display-4 fw-bold">12</div>
                      <div className="small opacity-75">测试用例</div>
                    </div>
                  </div>
                </div>
                <div className="col-md-3">
                  <div className="card bg-primary text-white h-100">
                    <div className="card-body text-center">
                      <div className="display-4 fw-bold">100%</div>
                      <div className="small opacity-75">功能覆盖</div>
                    </div>
                  </div>
                </div>
              </div>

              <div className="alert alert-primary mb-5">
                <h3 className="h5 alert-heading">🚀 快速开始</h3>
                <div className="d-flex flex-wrap gap-2 mt-3">
                  <a href="/mysql-demo.tsx" className="btn btn-primary">📊 演示仪表板</a>
                  <a href="/mysql-advanced.tsx" className="btn btn-primary">🔬 高级功能</a>
                  <a href="/mysql-performance.tsx" className="btn btn-primary">⚡ 性能测试</a>
                  <a href="/test-mysql" className="btn btn-outline-primary">🔗 简单 API</a>
                </div>
              </div>

              <h2 className="h4 border-bottom pb-2 mb-4">📚 主要功能页面</h2>
              <div className="row g-4 mb-5">
                <div className="col-md-6 col-lg-4">
                  <a href="/mysql-demo.tsx" className="card text-decoration-none text-dark h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">📊 MySQL 演示仪表板</h3>
                      <p className="card-text small text-muted">交互式演示仪表板，包含所有基本功能的可视化展示和测试。适合快速了解 MySQL 客户端的功能。</p>
                      <div>
                        <span className="badge bg-primary me-1">推荐</span>
                        <span className="badge bg-success me-1">交互式</span>
                        <span className="badge bg-secondary">入门</span>
                      </div>
                    </div>
                  </a>
                </div>

                <div className="col-md-6 col-lg-4">
                  <a href="/mysql-advanced.tsx" className="card text-decoration-none text-dark h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">🔬 高级功能测试</h3>
                      <p className="card-text small text-muted">12 种高级功能测试，包括批量操作、复杂查询、事务处理等。适合深入学习 MySQL 客户端的高级特性。</p>
                      <div>
                        <span className="badge bg-warning text-dark me-1">高级</span>
                        <span className="badge bg-secondary">全面</span>
                      </div>
                    </div>
                  </a>
                </div>

                <div className="col-md-6 col-lg-4">
                  <a href="/mysql-performance.tsx" className="card text-decoration-none text-dark h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">⚡ 性能测试</h3>
                      <p className="card-text small text-muted">MySQL 客户端性能基准测试，包括连接速度、查询性能、批量操作等指标。适合性能优化参考。</p>
                      <div>
                        <span className="badge bg-danger me-1">性能</span>
                        <span className="badge bg-secondary">基准测试</span>
                      </div>
                    </div>
                  </a>
                </div>

                <div className="col-md-6 col-lg-4">
                  <a href="/test-mysql" className="card text-decoration-none text-dark h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">🔗 JSON API 示例</h3>
                      <p className="card-text small text-muted">简单的 RESTful API 示例，展示如何在 TSX 页面中使用 MySQL 客户端返回 JSON 数据。</p>
                      <div>
                        <span className="badge bg-success me-1">API</span>
                        <span className="badge bg-secondary">简单</span>
                      </div>
                    </div>
                  </a>
                </div>

                <div className="col-md-6 col-lg-4">
                  <a href="/test-mysql-insert" className="card text-decoration-none text-dark h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">➕ 插入操作测试</h3>
                      <p className="card-text small text-muted">专门测试 MySQL 客户端的 insert 功能，返回插入的记录 ID。</p>
                      <div>
                        <span className="badge bg-secondary me-1">CRUD</span>
                        <span className="badge bg-secondary">创建</span>
                      </div>
                    </div>
                  </a>
                </div>

                <div className="col-md-6 col-lg-4">
                  <a href="/test-mysql-update" className="card text-decoration-none text-dark h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">✏️ 更新操作测试</h3>
                      <p className="card-text small text-muted">专门测试 MySQL 客户端的 update 功能，返回影响的行数。</p>
                      <div>
                        <span className="badge bg-secondary me-1">CRUD</span>
                        <span className="badge bg-secondary">更新</span>
                      </div>
                    </div>
                  </a>
                </div>

                <div className="col-md-6 col-lg-4">
                  <a href="/test-mysql-transaction" className="card text-decoration-none text-dark h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">🔄 事务操作测试</h3>
                      <p className="card-text small text-muted">测试 MySQL 客户端的事务功能，包括 beginTransaction、commit 和 rollback。</p>
                      <div>
                        <span className="badge bg-warning text-dark me-1">高级</span>
                        <span className="badge bg-secondary">事务</span>
                      </div>
                    </div>
                  </a>
                </div>
              </div>

              <h2 className="h4 border-bottom pb-2 mb-4">📖 功能特性</h2>
              <div className="row g-4 mb-5">
                <div className="col-md-6 col-lg-3">
                  <div className="card bg-light h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">🔗 连接管理</h3>
                      <p className="card-text small">支持连接池配置，自动管理连接的创建、复用和释放。提供灵活的连接参数配置。</p>
                      <div>
                        <span className="badge bg-primary">连接池</span>
                        <span className="badge bg-secondary ms-1">自动管理</span>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="col-md-6 col-lg-3">
                  <div className="card bg-light h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">📊 数据查询</h3>
                      <p className="card-text small">支持基本查询和参数化查询，防止 SQL 注入。支持 LIMIT、ORDER BY 等子句。</p>
                      <div>
                        <span className="badge bg-success">安全</span>
                        <span className="badge bg-secondary ms-1">防注入</span>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="col-md-6 col-lg-3">
                  <div className="card bg-light h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">✏️ 数据操作</h3>
                      <p className="card-text small">完整的 CRUD 操作支持，包括 insert、update、delete。方法简洁易用。</p>
                      <div>
                        <span className="badge bg-secondary">CRUD</span>
                        <span className="badge bg-secondary ms-1">完整</span>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="col-md-6 col-lg-3">
                  <div className="card bg-light h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title text-primary">🔄 事务支持</h3>
                      <p className="card-text small">完整的事务操作支持，包括 beginTransaction、commit、rollback。确保数据一致性。</p>
                      <div>
                        <span className="badge bg-warning text-dark">ACID</span>
                        <span className="badge bg-secondary ms-1">一致性</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <h2 className="h4 border-bottom pb-2 mb-4">🎯 测试场景</h2>
              <div className="row g-4 mb-5">
                <div className="col-md-3">
                  <div className="card bg-light h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title">基本查询</h3>
                      <p className="card-text small font-monospace bg-white p-2 rounded">SELECT * FROM users</p>
                      <span className="badge bg-success">简单</span>
                    </div>
                  </div>
                </div>

                <div className="col-md-3">
                  <div className="card bg-light h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title">参数化查询</h3>
                      <p className="card-text small font-monospace bg-white p-2 rounded">SELECT * FROM users WHERE id = ?</p>
                      <span className="badge bg-primary">安全</span>
                    </div>
                  </div>
                </div>

                <div className="col-md-3">
                  <div className="card bg-light h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title">批量操作</h3>
                      <p className="card-text small font-monospace bg-white p-2 rounded">使用事务批量插入 100 条记录</p>
                      <span className="badge bg-warning text-dark">高效</span>
                    </div>
                  </div>
                </div>

                <div className="col-md-3">
                  <div className="card bg-light h-100">
                    <div className="card-body">
                      <h3 className="h5 card-title">事务回滚</h3>
                      <p className="card-text small font-monospace bg-white p-2 rounded">测试失败时的事务回滚机制</p>
                      <span className="badge bg-danger">可靠性</span>
                    </div>
                  </div>
                </div>
              </div>

              <div className="text-center py-4 border-top">
                <p className="mb-2">
                  <strong>TSP MySQL 客户端</strong> - 类似 PHP 的数据库使用方式
                </p>
                <p className="text-muted small mb-0">
                  <a href="/" className="text-decoration-none text-primary me-3">返回首页</a>
                  <a href="/MYSQL_INTEGRATION.md" className="text-decoration-none text-primary me-3">查看文档</a>
                  <a href="/mysql-demo.tsx" className="text-decoration-none text-primary">演示仪表板</a>
                </p>
              </div>
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
          <meta name="viewport" content="width=device-width, initial-scale=1" />
          <link href="/static/css/bootstrap.min.css" rel="stylesheet" />
        </head>
        <body className="container py-5">
          <div className="row justify-content-center">
            <div className="col-md-8">
              <div className="alert alert-danger">
                <h1 className="h4">❌ 数据库连接失败</h1>
                <p className="mb-3">无法连接到 MySQL 数据库，请确保：</p>
                <ul>
                  <li>MySQL Docker 容器正在运行</li>
                  <li>数据库连接配置正确</li>
                  <li>网络连接正常</li>
                </ul>
                <p className="mb-0">运行 <code>.\docker-start.ps1</code> 启动 MySQL</p>
                <hr />
                <a href="/" className="btn btn-primary mt-3">返回首页</a>
              </div>
            </div>
          </div>
        </body>
      </html>
    );
  }
});
