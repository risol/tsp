/**
 * Header Component - Test JSX Component Import
 */

export interface HeaderProps {
  title: string;
  subtitle?: string;
}

export function Header(props: HeaderProps) {
  return (
    <header
      style={{
        background: "linear-gradient(135deg, #667eea 0%, #764ba2 100%)",
        color: "white",
        padding: "20px",
        borderRadius: "8px",
        marginBottom: "20px",
      }}
    >
      <h1 style={{ margin: 0, fontSize: "32px" }}>
        {props.title}
      </h1>
      {props.subtitle && (
        <p style={{ margin: "8px 0 0 0", opacity: 0.9 }}>
          {props.subtitle}
        </p>
      )}
    </header>
  );
}

export default Header;
