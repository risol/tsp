/**
 * 页脚组件 - Bootstrap工具类
 */
export function Footer() {
  const currentYear = new Date().getFullYear();
  return (
    <footer className="mt-5 py-5 text-center bg-white bg-opacity-10 rounded-3">
      <div className="mb-4">
        <h3 className="fs-4 mb-3 text-white">TSP</h3>
        <p className="fs-6 opacity-90">
          TypeScript Server Page - 高性能模板服务器
        </p>
      </div>

      <div className="d-flex justify-content-center gap-4 mb-4 flex-wrap">
        <a
          href="/"
          className="text-white text-decoration-none opacity-75"
        >
          首页
        </a>
        <a
          href="/demos"
          className="text-white text-decoration-none opacity-75"
        >
          演示
        </a>
        <a
          href="/features"
          className="text-white text-decoration-none opacity-75"
        >
          功能
        </a>
        <a
          href="/session_demo"
          className="text-white text-decoration-none opacity-75"
        >
          Session
        </a>
        <a
          href="/cookie_demo"
          className="text-white text-decoration-none opacity-75"
        >
          Cookies
        </a>
      </div>

      <div className="fs-6 opacity-75">
        <p className="mb-2">
          © {currentYear} TSP. Powered by{" "}
          <a
            href="https://deno.com"
            target="_blank"
            className="text-white text-decoration-underline"
          >
            Deno
          </a>
          {" + "}
          <a
            href="https://preactjs.com"
            target="_blank"
            className="text-white text-decoration-underline"
          >
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
