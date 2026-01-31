export function Footer() {
  const currentYear = new Date().getFullYear();
  return (
    <footer
      style={{
        marginTop: "60px",
        padding: "40px 20px",
        background: "rgba(255, 255, 255, 0.1)",
        borderRadius: "12px",
        textAlign: "center",
        color: "rgba(255, 255, 255, 0.9)",
      }}
    >
      <div style={{ marginBottom: "20px" }}>
        <h3 style={{ fontSize: "24px", marginBottom: "12px", color: "white" }}>
          TSP
        </h3>
        <p style={{ fontSize: "14px", opacity: 0.9 }}>
          TypeScript Server Page - 高性能模板服务器
        </p>
      </div>

      <div
        style={{
          display: "flex",
          justifyContent: "center",
          gap: "24px",
          marginBottom: "24px",
          flexWrap: "wrap",
        }}
      >
        <a
          href="/"
          style={{ color: "white", textDecoration: "none", opacity: 0.8 }}
        >
          首页
        </a>
        <a
          href="/demos"
          style={{ color: "white", textDecoration: "none", opacity: 0.8 }}
        >
          演示
        </a>
        <a
          href="/features"
          style={{ color: "white", textDecoration: "none", opacity: 0.8 }}
        >
          功能
        </a>
        <a
          href="/session_demo"
          style={{ color: "white", textDecoration: "none", opacity: 0.8 }}
        >
          Session
        </a>
        <a
          href="/cookie_demo"
          style={{ color: "white", textDecoration: "none", opacity: 0.8 }}
        >
          Cookies
        </a>
      </div>

      <div style={{ fontSize: "13px", opacity: 0.7 }}>
        <p style={{ marginBottom: "8px" }}>
          © {currentYear} TSP. Powered by{" "}
          <a
            href="https://deno.com"
            target="_blank"
            style={{ color: "white", textDecoration: "underline" }}
          >
            Deno
          </a>
          {" + "}
          <a
            href="https://preactjs.com"
            target="_blank"
            style={{ color: "white", textDecoration: "underline" }}
          >
            Preact
          </a>
        </p>
        <p>
          基于 TypeScript + JSX 构建的现代化 Web 框架
        </p>
      </div>
    </footer>
  );
}

export default Footer;
