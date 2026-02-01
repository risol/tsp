/**
 * LDAP 演示页面
 * 展示如何使用 AppDeps 注入的 LDAP 客户端
 */

import { Layout } from "./components/Layout.tsx";

export default Page(async function (ctx, { createLdap, response }) {
  const { url } = ctx;

  // 获取操作类型
  const action = url.searchParams.get("action") || "info";
  const message = url.searchParams.get("message") || "";
  const error = url.searchParams.get("error") || "";

  // 处理 POST 请求
  if (ctx.method === "POST") {
    try {
      const formData = ctx.body as Record<string, string>;
      const ldapAction = formData.action;

      // LDAP 配置
      const ldapConfig = {
        url: formData.ldapUrl || "ldap://localhost:389",
        bindDN: formData.bindDN || "",
        bindCredentials: formData.password || "",
        baseDN: formData.baseDN || "dc=example,dc=org",
        startTLS: formData.startTLS === "true",
      };

      const ldap = await createLdap(ldapConfig);

      if (ldapAction === "bind") {
        // 测试绑定
        await ldap.bind(formData.bindDN!, formData.password!);
        await ldap.close();
        return response.redirect(
          `/ldap-demo?action=info&message=${encodeURIComponent("绑定成功！")}`
        );
      } else if (ldapAction === "search") {
        // 搜索条目
        const entries = await ldap.search(
          formData.searchBase || ldapConfig.baseDN,
          {
            filter: formData.searchFilter || "(objectClass=*)",
            scope: (formData.searchScope as "base" | "one" | "sub") || "sub",
            attributes: formData.attributes
              ? formData.attributes.split(",")
              : undefined,
          }
        );

        await ldap.close();

        return response.json({
          success: true,
          count: entries.length,
          entries: entries.map((entry) => ({
            dn: entry.dn,
            attributes: entry.attributes,
          })),
        });
      } else if (ldapAction === "authenticate") {
        // 用户认证
        const userDN = formData.userDN;
        const password = formData.userPassword;

        if (!userDN || !password) {
          return response.redirect(
            `/ldap-demo?action=authenticate&error=${encodeURIComponent("缺少用户DN或密码")}`
          );
        }

        try {
          await ldap.bind(userDN, password);
          await ldap.close();
          return response.redirect(
            `/ldap-demo?action=authenticate&message=${encodeURIComponent("认证成功！用户：")}${userDN}`
          );
        } catch (err) {
          await ldap.close();
          return response.redirect(
            `/ldap-demo?action=authenticate&error=${encodeURIComponent("认证失败：" + (err as Error).message)}`
          );
        }
      }

      await ldap.close();
    } catch (err) {
      return response.redirect(
        `/ldap-demo?action=${action}&error=${encodeURIComponent((err as Error).message)}`
      );
    }
  }

  // 渲染页面
  return (
    <Layout title="LDAP 演示" description="展示如何使用 TSP 的 LDAP 客户端">
      <div className="container py-5">
        <div className="row">
          <div className="col-lg-8 mx-auto">
            <h1 className="mb-4">🔐 LDAP 演示</h1>

            {/* 消息提示 */}
            {message && (
              <div className="alert alert-success alert-dismissible fade show" role="alert">
                {message}
                <button
                  type="button"
                  className="btn-close"
                  data-bs-dismiss="alert"
                  aria-label="Close"
                >
                </button>
              </div>
            )}

            {error && (
              <div className="alert alert-danger alert-dismissible fade show" role="alert">
                <strong>错误：</strong> {error}
                <button
                  type="button"
                  className="btn-close"
                  data-bs-dismiss="alert"
                  aria-label="Close"
                >
                </button>
              </div>
            )}

            {/* 导航标签 */}
            <ul className="nav nav-tabs mb-4" id="ldapTab" role="tablist">
              <li className="nav-item">
                <button
                  className={`nav-link ${action === "info" ? "active" : ""}`}
                  data-bs-toggle="tab"
                  data-bs-target="#info"
                  type="button"
                >
                  说明
                </button>
              </li>
              <li className="nav-item">
                <button
                  className={`nav-link ${action === "authenticate" ? "active" : ""}`}
                  data-bs-toggle="tab"
                  data-bs-target="#authenticate"
                  type="button"
                >
                  用户认证
                </button>
              </li>
              <li className="nav-item">
                <button
                  className={`nav-link ${action === "search" ? "active" : ""}`}
                  data-bs-toggle="tab"
                  data-bs-target="#search"
                  type="button"
                >
                  搜索条目
                </button>
              </li>
            </ul>

            {/* 标签内容 */}
            <div className="tab-content" id="ldapTabContent">
              {/* 说明页面 */}
              <div
                className={`tab-pane fade ${action === "info" ? "show active" : ""}`}
                id="info"
              >
                <div className="card">
                  <div className="card-body">
                    <h5 className="card-title">什么是 LDAP？</h5>
                    <p>
                      LDAP（Lightweight Directory Access Protocol，轻量级目录访问协议）
                      是一种用于访问和维护分布式目录信息的协议。
                    </p>

                    <h6 className="mt-4">常见用途：</h6>
                    <ul>
                      <li>用户认证（Single Sign-On）</li>
                      <li>组织架构管理</li>
                      <li>电子邮件目录</li>
                      <li>地址簿服务</li>
                    </ul>

                    <h6 className="mt-4">TSP 中的 LDAP 使用：</h6>
                    <pre className="bg-light p-3 rounded">
{`export default Page(async function(ctx, { createLdap }) {
  // 创建 LDAP 客户端（Docker 测试环境）
  const ldap = await createLdap({
    url: 'ldap://localhost:1389',  // 注意：Docker 映射端口
    bindDN: 'cn=admin,dc=example,dc=org',
    bindCredentials: 'admin123456',
    baseDN: 'dc=example,dc=org'
  });

  // 搜索用户
  const entries = await ldap.search('ou=developers,dc=example,dc=org', {
    filter: '(objectClass=person)',
    scope: 'sub'
  });

  // 用户认证
  await ldap.bind('cn=zhang san,ou=developers,dc=example,dc=org', 'password123');

  await ldap.close();
});`}
                    </pre>

                    <h6 className="mt-4">Docker 测试环境信息：</h6>
                    <div className="alert alert-info">
                      <strong>测试服务器：</strong> ldap://localhost:1389<br/>
                      <strong>管理员：</strong> cn=admin,dc=example,dc=org / admin123456<br/>
                      <strong>测试用户（6个）：</strong><br/>
                      - 张三: cn=zhang san,ou=developers,dc=example,dc=org / password123<br/>
                      - 李四: cn=li si,ou=developers,dc=example,dc=org / password456<br/>
                      - 王五: cn=wang wu,ou=developers,dc=example,dc=org / password789<br/>
                      - user01-03: cn=user01,ou=developers,dc=example,dc=org / password01-03
                    </div>

                    <h6 className="mt-4">配置说明：</h6>
                    <table className="table table-sm">
                      <thead>
                        <tr>
                          <th>参数</th>
                          <th>说明</th>
                          <th>示例</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr>
                          <td><code>url</code></td>
                          <td>LDAP 服务器地址</td>
                          <td><code>ldap://localhost:1389</code> (Docker)</td>
                        </tr>
                        <tr>
                          <td><code>bindDN</code></td>
                          <td>管理员 DN</td>
                          <td><code>cn=admin,dc=example,dc=org</code></td>
                        </tr>
                        <tr>
                          <td><code>bindCredentials</code></td>
                          <td>管理员密码</td>
                          <td><code>admin123456</code></td>
                        </tr>
                        <tr>
                          <td><code>baseDN</code></td>
                          <td>搜索基准 DN</td>
                          <td><code>ou=developers,dc=example,dc=org</code></td>
                        </tr>
                        <tr>
                          <td><code>startTLS</code></td>
                          <td>启用 TLS 加密</td>
                          <td><code>true</code> / <code>false</code></td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>

              {/* 用户认证 */}
              <div
                className={`tab-pane fade ${action === "authenticate" ? "show active" : ""}`}
                id="authenticate"
              >
                <div className="card">
                  <div className="card-body">
                    <h5 className="card-title">用户认证</h5>
                    <p className="text-muted">使用 LDAP 验证用户凭据</p>

                    <form method="POST">
                      <input type="hidden" name="action" value="authenticate" />

                      <div className="mb-3">
                        <label htmlFor="ldapUrl" className="form-label">
                          LDAP 服务器
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="ldapUrl"
                          name="ldapUrl"
                          defaultValue="ldap://localhost:1389"
                          required
                        />
                      </div>

                      <div className="mb-3">
                        <label htmlFor="bindDN" className="form-label">
                          管理员 DN（可选）
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="bindDN"
                          name="bindDN"
                          placeholder="cn=admin,dc=example,dc=org"
                        />
                        <div className="form-text">
                          留空则使用用户凭据直接绑定
                        </div>
                      </div>

                      <div className="mb-3">
                        <label htmlFor="password" className="form-label">
                          管理员密码（可选）
                        </label>
                        <input
                          type="password"
                          className="form-control"
                          id="password"
                          name="password"
                        />
                      </div>

                      <hr />

                      <div className="mb-3">
                        <label htmlFor="userDN" className="form-label">
                          用户 DN
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="userDN"
                          name="userDN"
                          placeholder="cn=user,dc=example,dc=org"
                          list="testUsers"
                          required
                        />
                        <datalist id="testUsers">
                          <option value="cn=zhang san,ou=developers,dc=example,dc=org">张三 (password123)</option>
                          <option value="cn=li si,ou=developers,dc=example,dc=org">李四 (password456)</option>
                          <option value="cn=wang wu,ou=developers,dc=example,dc=org">王五 (password789)</option>
                          <option value="cn=user01,ou=developers,dc=example,dc=org">user01 (password01)</option>
                          <option value="cn=user02,ou=developers,dc=example,dc=org">user02 (password02)</option>
                          <option value="cn=user03,ou=developers,dc=example,dc=org">user03 (password03)</option>
                        </datalist>
                        <div className="form-text">
                          测试用户：从下拉列表选择或手动输入
                        </div>
                      </div>

                      <div className="mb-3">
                        <label htmlFor="userPassword" className="form-label">
                          用户密码
                        </label>
                        <input
                          type="password"
                          className="form-control"
                          id="userPassword"
                          name="userPassword"
                          placeholder="password123"
                          required
                        />
                        <div className="form-text">
                          测试用户密码：张三=password123, 李四=password456, 王五=password789, user01=password01, user02=password02, user03=password03
                        </div>
                      </div>

                      <button type="submit" className="btn btn-primary">
                        认证
                      </button>
                    </form>
                  </div>
                </div>
              </div>

              {/* 搜索条目 */}
              <div
                className={`tab-pane fade ${action === "search" ? "show active" : ""}`}
                id="search"
              >
                <div className="card">
                  <div className="card-body">
                    <h5 className="card-title">搜索条目</h5>
                    <p className="text-muted">在 LDAP 目录中搜索条目</p>

                    <form
                      method="POST"
                      id="searchForm"
                      // biome-ignore lint: need onsubmit attribute for inline JS
                      {...({ onsubmit: "handleSearch(event)" } as Record<string, string>)}
                    >
                      <input type="hidden" name="action" value="search" />

                      <div className="mb-3">
                        <label htmlFor="ldapUrl" className="form-label">
                          LDAP 服务器
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="ldapUrl"
                          name="ldapUrl"
                          defaultValue="ldap://localhost:1389"
                          required
                        />
                      </div>

                      <div className="mb-3">
                        <label htmlFor="bindDN" className="form-label">
                          绑定 DN
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="bindDN"
                          name="bindDN"
                          defaultValue="cn=admin,dc=example,dc=org"
                          required
                        />
                      </div>

                      <div className="mb-3">
                        <label htmlFor="password" className="form-label">
                          密码
                        </label>
                        <input
                          type="password"
                          className="form-control"
                          id="password"
                          name="password"
                          defaultValue="admin123456"
                          required
                        />
                      </div>

                      <div className="mb-3">
                        <label htmlFor="baseDN" className="form-label">
                          基准 DN
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="baseDN"
                          name="baseDN"
                          defaultValue="dc=example,dc=org"
                          required
                        />
                      </div>

                      <div className="mb-3">
                        <label htmlFor="searchBase" className="form-label">
                          搜索基准
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="searchBase"
                          name="searchBase"
                          defaultValue="ou=developers,dc=example,dc=org"
                          required
                        />
                        <div className="form-text">
                          测试用户在 ou=developers,dc=example,dc=org 下（6个用户）
                        </div>
                      </div>

                      <div className="mb-3">
                        <label htmlFor="searchFilter" className="form-label">
                          过滤器
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="searchFilter"
                          name="searchFilter"
                          defaultValue="(objectClass=person)"
                          required
                        />
                        <div className="form-text">
                          常用过滤器：(objectClass=*) 所有条目，(objectClass=person) 人员，(cn=zhang san) 特定用户
                        </div>
                      </div>

                      <div className="mb-3">
                        <label htmlFor="searchScope" className="form-label">
                          搜索范围
                        </label>
                        <select className="form-select" id="searchScope" name="searchScope">
                          <option value="base">Base（仅基准）</option>
                          <option value="one">One（一级）</option>
                          <option value="sub" selected>Sub（子树）</option>
                        </select>
                      </div>

                      <div className="mb-3">
                        <label htmlFor="attributes" className="form-label">
                          返回属性（逗号分隔，留空返回所有）
                        </label>
                        <input
                          type="text"
                          className="form-control"
                          id="attributes"
                          name="attributes"
                          placeholder="cn,mail,uid"
                        />
                      </div>

                      <button type="submit" className="btn btn-primary">
                        搜索
                      </button>
                    </form>

                    <div id="searchResult" className="mt-4"></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <script dangerouslySetInnerHTML={{
        __html: `
          function handleSearch(event) {
            event.preventDefault();
            const form = event.target;
            const formData = new FormData(form);
            const resultDiv = document.getElementById('searchResult');

            resultDiv.innerHTML = '<div class="spinner-border" role="status"><span class="visually-hidden">Loading...</span></div>';

            fetch('/ldap-demo', {
              method: 'POST',
              headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
              },
              body: new URLSearchParams(formData).toString()
            })
            .then(res => res.json())
            .then(data => {
              if (data.success) {
                let html = '<div class="alert alert-success">找到 ' + data.count + ' 个条目</div>';
                html += '<div class="list-group">';
                data.entries.forEach(entry => {
                  html += '<div class="list-group-item">';
                  html += '<strong>' + entry.dn + '</strong>';
                  html += '<pre class="mb-0 mt-2">' + JSON.stringify(entry.attributes, null, 2) + '</pre>';
                  html += '</div>';
                });
                html += '</div>';
                resultDiv.innerHTML = html;
              } else {
                resultDiv.innerHTML = '<div class="alert alert-danger">搜索失败</div>';
              }
            })
            .catch(err => {
              resultDiv.innerHTML = '<div class="alert alert-danger">错误: ' + err.message + '</div>';
            });
          }
        `
      }} />
    </Layout>
  );
});
