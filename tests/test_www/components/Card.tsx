/**
 * Card 组件 - 测试 JSX 组件导入和嵌套
 */

export interface CardProps {
  title: string;
  content: string;
  footer?: string;
}

export function Card(props: CardProps) {
  return (
    <div
      style={{
        border: "1px solid #e2e8f0",
        borderRadius: "8px",
        padding: "20px",
        marginBottom: "16px",
        backgroundColor: "#f8fafc",
      }}
    >
      <h3 style={{ margin: "0 0 12px 0", color: "#667eea" }}>
        {props.title}
      </h3>
      <p style={{ margin: "0 0 16px 0", color: "#64748b" }}>
        {props.content}
      </p>
      {props.footer && (
        <div
          style={{
            borderTop: "1px solid #e2e8f0",
            paddingTop: "12px",
            fontSize: "14px",
            color: "#94a3b8",
          }}
        >
          {props.footer}
        </div>
      )}
    </div>
  );
}

export default Card;
