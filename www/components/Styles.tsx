/**
 * 自定义样式组件
 * 只保留Bootstrap无法提供的自定义样式
 */

/**
 * 获取自定义样式
 * Bootstrap已提供大部分基础样式，这里只保留特殊的自定义样式
 */
export function getCustomStyles(): string {
  return `
    /* 品牌渐变背景 */
    .bg-gradient-brand {
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%) !important;
      min-height: 100vh;
    }

    /* 代码块样式（Bootstrap没有深色代码块） */
    .code-block {
      background: #1e293b;
      color: #e2e8f0;
      padding: 1rem;
      border-radius: 0.375rem;
      font-family: 'Courier New', monospace;
      font-size: 0.875rem;
      line-height: 1.5;
      overflow-x: auto;
    }

    /* 信息项样式 */
    .info-item {
      padding: 0.75rem;
      background: #f8fafc;
      border-left: 3px solid #667eea;
      border-radius: 0.25rem;
    }

    .info-label {
      font-weight: 600;
      color: #667eea;
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .info-value {
      margin-top: 0.25rem;
      word-break: break-all;
    }

    /* 信息网格布局 */
    .info-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
      gap: 1.5rem;
      margin-bottom: 2rem;
    }

    /* 分区样式 */
    .section {
      margin-bottom: 3rem;
    }

    .section-title {
      font-size: 1.75rem;
      font-weight: 600;
      color: #1e293b;
      margin-bottom: 1.5rem;
      padding-bottom: 0.75rem;
      border-bottom: 2px solid #e2e8f0;
    }

    /* 功能卡片样式 */
    .card {
      background: white;
      border-radius: 0.75rem;
      padding: 1.5rem;
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
      transition: transform 0.2s, box-shadow 0.2s;
      display: flex;
      flex-direction: column;
      height: 100%;
    }

    .card:hover {
      transform: translateY(-2px);
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    }

    /* htmx加载指示器 */
    .htmx-indicator {
      opacity: 0;
      transition: opacity 200ms ease-in;
    }
    .htmx-indicator.htmx-request {
      opacity: 1;
    }
    .htmx-indicator.d-none {
      display: none !important;
    }

    /* 淡入动画效果 */
    .fade-in {
      animation: fadeIn 0.3s ease-in;
    }

    @keyframes fadeIn {
      from {
        opacity: 0;
        transform: translateY(-10px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    /* 渐变背景 */
    .bg-gradient-primary {
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    }
  `;
}

/**
 * @deprecated 使用 getCustomStyles() 替代
 * 保留此函数以兼容旧代码
 */
export function getGlobalStyles(): string {
  return getCustomStyles();
}
