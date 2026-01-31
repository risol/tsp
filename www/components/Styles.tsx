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
  `;
}

/**
 * @deprecated 使用 getCustomStyles() 替代
 * 保留此函数以兼容旧代码
 */
export function getGlobalStyles(): string {
  return getCustomStyles();
}
