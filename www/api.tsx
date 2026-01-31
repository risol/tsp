import { Layout } from "./components/Layout.tsx";

export default async function (context: PageContext) {
  const { method, url, query, body, cookies, file, root, headers } = context;

  // 处理 headers
  const headersObj: Record<string, string> = {};
  for (const [key, value] of headers.entries()) {
    headersObj[key] = value;
  }

  const queryParams = Object.keys(query).length > 0
    ? JSON.stringify(query, null, 2)
    : "暂无查询参数";

  const bodyData = body ? JSON.stringify(body, null, 2) : "暂无 POST 数据";

  const cookiesData = Object.keys(cookies).length > 0
    ? JSON.stringify(cookies, null, 2)
    : "暂无 Cookies";

  return (
    <Layout title="API 信息 - TSP">
      <div className="container-fluid py-4">
        <h1 className="display-5 fw-bold mb-4">🔧 API 请求信息</h1>

        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">请求方法</h2>
            <pre className="code-block">{method}</pre>
          </div>
        </div>

        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">URL 信息</h2>
            <pre className="code-block">{JSON.stringify({
              href: url.href,
              origin: url.origin,
              pathname: url.pathname,
              search: url.search
            }, null, 2)}</pre>
          </div>
        </div>

        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">查询参数 (Query)</h2>
            <pre className="code-block">{queryParams}</pre>
          </div>
        </div>

        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">请求头 (Headers)</h2>
            <pre className="code-block">{JSON.stringify(headersObj, null, 2)}</pre>
          </div>
        </div>

        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">POST 数据 (Body)</h2>
            <pre className="code-block">{bodyData}</pre>
          </div>
        </div>

        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">Cookies</h2>
            <pre className="code-block">{cookiesData}</pre>
          </div>
        </div>

        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">文件路径</h2>
            <pre className="code-block">{JSON.stringify({
              template: file,
              root
            }, null, 2)}</pre>
          </div>
        </div>

        <div className="card shadow-sm">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">测试链接</h2>
            <ul className="list-group">
              <li className="list-group-item">
                <a className="text-decoration-none" href="?name=test&lang=zh">?name=test&lang=zh</a>
              </li>
              <li className="list-group-item">
                <a className="text-decoration-none" href="?debug=true&verbose=1">?debug=true&verbose=1</a>
              </li>
              <li className="list-group-item">
                <a className="text-decoration-none" href="/">返回首页</a>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </Layout>
  );
}
