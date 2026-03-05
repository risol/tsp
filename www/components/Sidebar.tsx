export function Sidebar() {
  return (
    <aside class="sidebar">
      <div style={{ marginBottom: "24px" }}>
        <a href="/" style={{ fontSize: "18px", fontWeight: "600", color: "#3178c6", textDecoration: "none" }}>TSP</a>
      </div>

      <div style={{ marginBottom: "20px" }}>
        <div style={{ fontSize: "12px", fontWeight: "600", color: "#586069", marginBottom: "8px", textTransform: "uppercase" }}>Getting Started</div>
        <ul style={{ listStyle: "none", padding: "0", margin: "0" }}>
          <li style={{ marginBottom: "4px" }}><a href="/" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px" }}>Introduction</a></li>
          <li style={{ marginBottom: "4px" }}><a href="/getting-started" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px", background: "#e8f5e9" }}>Quick Start</a></li>
        </ul>
      </div>

      <div style={{ marginBottom: "20px" }}>
        <div style={{ fontSize: "12px", fontWeight: "600", color: "#586069", marginBottom: "8px", textTransform: "uppercase" }}>Guide</div>
        <ul style={{ listStyle: "none", padding: "0", margin: "0" }}>
          <li style={{ marginBottom: "4px" }}><a href="/features" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px" }}>Features</a></li>
          <li style={{ marginBottom: "4px" }}><a href="/examples" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px" }}>Examples</a></li>
        </ul>
      </div>

      <div style={{ marginBottom: "20px" }}>
        <div style={{ fontSize: "12px", fontWeight: "600", color: "#586069", marginBottom: "8px", textTransform: "uppercase" }}>More</div>
        <ul style={{ listStyle: "none", padding: "0", margin: "0" }}>
          <li style={{ marginBottom: "4px" }}><a href="https://github.com/risol/tsp" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px" }}>GitHub</a></li>
        </ul>
      </div>

      <div style={{ marginTop: "auto", paddingTop: "20px", borderTop: "1px solid #e1e4e8" }}>
        <div style={{ fontSize: "12px", color: "#586069" }}>Version: 0.1.0</div>
      </div>
    </aside>
  );
}
