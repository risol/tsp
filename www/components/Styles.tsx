/**
 * 共享样式组件
 * 提供统一的 CSS 样式
 */

export function getGlobalStyles(): string {
  return `
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }

    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      line-height: 1.6;
      color: #333;
    }

    .container {
      max-width: 1200px;
      margin: 0 auto;
      padding: 0 20px;
    }

    .card {
      background: white;
      border-radius: 8px;
      padding: 24px;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
      margin-bottom: 20px;
    }

    .btn {
      display: inline-block;
      padding: 10px 20px;
      border: none;
      border-radius: 6px;
      font-size: 14px;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s ease;
      text-decoration: none;
    }

    .btn-primary {
      background: #667eea;
      color: white;
    }

    .btn-primary:hover {
      background: #5568d3;
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
    }

    .btn-secondary {
      background: #e2e8f0;
      color: #333;
    }

    .btn-secondary:hover {
      background: #cbd5e0;
    }

    .badge {
      display: inline-block;
      padding: 4px 12px;
      border-radius: 12px;
      font-size: 12px;
      font-weight: 600;
    }

    .badge-success {
      background: #d1fae5;
      color: #065f46;
    }

    .badge-info {
      background: #dbeafe;
      color: #1e40af;
    }

    .code-block {
      background: #1e293b;
      color: #e2e8f0;
      padding: 16px;
      border-radius: 6px;
      overflow-x: auto;
      font-family: 'Courier New', monospace;
      font-size: 13px;
      line-height: 1.5;
    }

    .info-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
      gap: 16px;
      margin: 20px 0;
    }

    .info-item {
      padding: 12px;
      background: #f8fafc;
      border-left: 3px solid #667eea;
      border-radius: 4px;
    }

    .info-label {
      font-weight: 600;
      color: #667eea;
      font-size: 12px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .info-value {
      margin-top: 4px;
      word-break: break-all;
    }

    .section {
      margin: 40px 0;
    }

    .section-title {
      font-size: 24px;
      font-weight: 700;
      margin-bottom: 16px;
      color: #1e293b;
    }

    table {
      width: 100%;
      border-collapse: collapse;
      margin: 20px 0;
    }

    table th,
    table td {
      padding: 12px;
      text-align: left;
      border-bottom: 1px solid #e2e8f0;
    }

    table th {
      background: #f8fafc;
      font-weight: 600;
      color: #475569;
    }

    table tr:hover {
      background: #f8fafc;
    }
  `;
}
