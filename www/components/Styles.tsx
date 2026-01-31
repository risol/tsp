/**
 * 自定义样式组件 - 专业级视觉效果
 * 提供Bootstrap无法提供的自定义样式和动画效果
 */

/**
 * 获取自定义样式
 * Bootstrap已提供大部分基础样式，这里提供增强的视觉效果
 */
export function getCustomStyles(): string {
  return `
    /* ===== 基础样式 ===== */

    /* 品牌渐变背景 - 更专业的蓝色调 */
    .bg-gradient-brand {
      background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%) !important;
      min-height: 100vh;
    }

    /* 渐变导航栏 - 更专业的蓝色调 */
    .navbar-gradient {
      background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%) !important;
      box-shadow: 0 2px 20px rgba(30, 64, 175, 0.3);
    }

    /* ===== 卡片样式增强 ===== */

    /* 代码块样式（专业深色主题） */
    .code-block {
      background: #0f172a;
      color: #e2e8f0;
      padding: 1.5rem;
      border-radius: 0.75rem;
      font-family: 'Consolas', 'Monaco', monospace;
      font-size: 0.875rem;
      line-height: 1.7;
      overflow-x: auto;
      border: 1px solid rgba(255, 255, 255, 0.1);
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    }

    /* 信息项样式 - 更专业的设计 */
    .info-item {
      padding: 1.25rem;
      background: rgba(255, 255, 255, 0.95);
      border-left: 4px solid #3b82f6;
      border-radius: 0.75rem;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
      transition: all 0.3s ease;
    }

    .info-item:hover {
      transform: translateX(4px);
      box-shadow: 0 4px 12px rgba(59, 130, 246, 0.2);
    }

    .info-label {
      font-weight: 600;
      color: #3b82f6;
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .info-value {
      margin-top: 0.25rem;
      word-break: break-all;
      font-weight: 500;
    }

    /* ===== 网格和布局 ===== */

    /* 信息网格布局 */
    .info-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 1.5rem;
      margin-bottom: 2rem;
    }

    /* 分区样式 - 更专业的半透明效果 */
    .section {
      margin-bottom: 3rem;
      padding: 1.5rem;
      background: rgba(255, 255, 255, 0.6);
      border-radius: 1.25rem;
      backdrop-filter: blur(10px);
      border: 1px solid rgba(255, 255, 255, 0.2);
    }

    .section-title {
      font-size: 2rem;
      font-weight: 700;
      background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      margin-bottom: 1.5rem;
      padding-bottom: 0.75rem;
      border-bottom: 3px solid;
      border-image: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%) 1;
    }

    /* 功能卡片样式 - 更专业的设计 */
    .feature-card {
      background: rgba(255, 255, 255, 0.95);
      border-radius: 1.25rem;
      padding: 2rem;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.07);
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      display: flex;
      flex-direction: column;
      height: 100%;
      border: 1px solid rgba(59, 130, 246, 0.1);
    }

    .feature-card:hover {
      transform: translateY(-8px);
      box-shadow: 0 20px 25px rgba(59, 130, 246, 0.15);
      border-color: rgba(59, 130, 246, 0.3);
    }

    /* Hero 区域增强 - 更专业的蓝色调 */
    .hero-section {
      background: linear-gradient(135deg, rgba(30, 64, 175, 0.95) 0%, rgba(59, 130, 246, 0.95) 100%);
      border-radius: 2rem;
      padding: 4rem 2rem;
      text-align: center;
      color: white;
      box-shadow: 0 10px 40px rgba(30, 64, 175, 0.3);
      margin-bottom: 3rem;
      position: relative;
      overflow: hidden;
    }

    .hero-section::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: radial-gradient(circle at 20% 50%, rgba(255, 255, 255, 0.1) 0%, transparent 50%);
      animation: pulse 3s ease-in-out infinite;
    }

    .hero-title {
      font-size: 4rem;
      font-weight: 800;
      margin-bottom: 1rem;
      text-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
      position: relative;
      z-index: 1;
    }

    .hero-subtitle {
      font-size: 1.75rem;
      font-weight: 500;
      opacity: 0.95;
      margin-bottom: 0.5rem;
      position: relative;
      z-index: 1;
    }

    .hero-description {
      font-size: 1.25rem;
      opacity: 0.85;
      margin-bottom: 2rem;
      position: relative;
      z-index: 1;
    }

    /* ===== 按钮增强 ===== */

    .btn-hero {
      padding: 1rem 2.5rem;
      font-size: 1.125rem;
      font-weight: 600;
      border-radius: 0.75rem;
      transition: all 0.3s ease;
      box-shadow: 0 4px 14px rgba(0, 0, 0, 0.15);
    }

    .btn-hero:hover {
      transform: translateY(-2px);
      box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
    }

    .btn-gradient-primary {
      background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%);
      border: none;
      color: white;
      transition: all 0.3s ease;
    }

    .btn-gradient-primary:hover {
      background: linear-gradient(135deg, #1e3a8a 0%, #2563eb 100%);
      transform: translateY(-2px);
      box-shadow: 0 8px 20px rgba(59, 130, 246, 0.4);
    }

    /* ===== 图标动画 ===== */

    .icon-float {
      animation: float 3s ease-in-out infinite;
    }

    @keyframes float {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-10px); }
    }

    /* ===== 导航增强 ===== */

    .navbar-brand-custom {
      font-size: 1.75rem;
      font-weight: 800;
      letter-spacing: -0.5px;
      transition: all 0.3s ease;
    }

    .navbar-brand-custom:hover {
      transform: scale(1.05);
    }

    .nav-link-custom {
      font-weight: 500;
      padding: 0.5rem 1rem !important;
      border-radius: 0.375rem;
      transition: all 0.3s ease;
      position: relative;
    }

    .nav-link-custom::after {
      content: '';
      position: absolute;
      bottom: 0;
      left: 50%;
      width: 0;
      height: 2px;
      background: white;
      transition: all 0.3s ease;
      transform: translateX(-50%);
    }

    .nav-link-custom:hover::after {
      width: 80%;
    }

    .nav-link-custom:hover {
      background: rgba(255, 255, 255, 0.1);
    }

    /* ===== 统计卡片 ===== */

    .stat-card {
      background: rgba(255, 255, 255, 0.95);
      border-radius: 1.25rem;
      padding: 2rem;
      text-align: center;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.07);
      transition: all 0.3s ease;
      border: 1px solid rgba(59, 130, 246, 0.1);
    }

    .stat-card:hover {
      transform: translateY(-4px);
      box-shadow: 0 12px 24px rgba(59, 130, 246, 0.15);
    }

    .stat-number {
      font-size: 3rem;
      font-weight: 800;
      background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }

    .stat-label {
      font-size: 0.875rem;
      font-weight: 600;
      color: #64748b;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    /* ===== 动画效果 ===== */

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

    .fade-in {
      animation: fadeIn 0.5s ease-in;
    }

    @keyframes fadeIn {
      from {
        opacity: 0;
        transform: translateY(-20px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    .slide-in-left {
      animation: slideInLeft 0.6s ease-out;
    }

    @keyframes slideInLeft {
      from {
        opacity: 0;
        transform: translateX(-30px);
      }
      to {
        opacity: 1;
        transform: translateX(0);
      }
    }

    .scale-in {
      animation: scaleIn 0.4s ease-out;
    }

    @keyframes scaleIn {
      from {
        opacity: 0;
        transform: scale(0.9);
      }
      to {
        opacity: 1;
        transform: scale(1);
      }
    }

    @keyframes pulse {
      0%, 100% { opacity: 0.3; }
      50% { opacity: 0.5; }
    }

    /* ===== 渐变背景变体 ===== */

    .bg-gradient-primary {
      background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%);
    }

    .bg-gradient-success {
      background: linear-gradient(135deg, #059669 0%, #10b981 100%);
    }

    .bg-gradient-info {
      background: linear-gradient(135deg, #0284c7 0%, #0ea5e9 100%);
    }

    .bg-gradient-warning {
      background: linear-gradient(135deg, #d97706 0%, #f59e0b 100%);
    }

    .bg-gradient-danger {
      background: linear-gradient(135deg, #dc2626 0%, #ef4444 100%);
    }

    /* ===== 页脚增强 ===== */

    .footer-enhanced {
      background: rgba(255, 255, 255, 0.1);
      backdrop-filter: blur(10px);
      border: 1px solid rgba(255, 255, 255, 0.2);
    }

    /* ===== 表格增强 ===== */

    .table-enhanced {
      background: white;
      border-radius: 1rem;
      overflow: hidden;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
    }

    .table-enhanced thead {
      background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%);
      color: white;
    }

    .table-enhanced tbody tr {
      transition: all 0.2s ease;
    }

    .table-enhanced tbody tr:hover {
      background: rgba(59, 130, 246, 0.05);
      transform: scale(1.01);
    }

    /* ===== 响应式调整 ===== */

    @media (max-width: 768px) {
      .hero-title {
        font-size: 2.5rem;
      }

      .hero-subtitle {
        font-size: 1.25rem;
      }

      .section-title {
        font-size: 1.5rem;
      }
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
