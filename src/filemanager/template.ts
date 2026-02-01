/**
 * 文件管理器前端界面模板
 * 生成登录页面和主界面
 */

import { formatFileSize, formatDateTime } from "./config.ts";

/**
 * 生成登录页面
 */
export function generateLoginPage(): string {
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>文件管理器 - 登录</title>
  <style>
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }

    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 20px;
    }

    .login-container {
      background: white;
      border-radius: 12px;
      box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
      padding: 40px;
      width: 100%;
      max-width: 400px;
    }

    .login-header {
      text-align: center;
      margin-bottom: 30px;
    }

    .login-header h1 {
      color: #333;
      font-size: 24px;
      margin-bottom: 8px;
    }

    .login-header p {
      color: #666;
      font-size: 14px;
    }

    .form-group {
      margin-bottom: 20px;
    }

    .form-group label {
      display: block;
      color: #333;
      font-size: 14px;
      font-weight: 500;
      margin-bottom: 8px;
    }

    .form-group input {
      width: 100%;
      padding: 12px 16px;
      border: 1px solid #ddd;
      border-radius: 6px;
      font-size: 14px;
      transition: border-color 0.3s;
    }

    .form-group input:focus {
      outline: none;
      border-color: #667eea;
    }

    .btn-login {
      width: 100%;
      padding: 12px;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      color: white;
      border: none;
      border-radius: 6px;
      font-size: 16px;
      font-weight: 500;
      cursor: pointer;
      transition: transform 0.2s, box-shadow 0.2s;
    }

    .btn-login:hover {
      transform: translateY(-2px);
      box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
    }

    .btn-login:active {
      transform: translateY(0);
    }

    .btn-login:disabled {
      opacity: 0.6;
      cursor: not-allowed;
      transform: none;
    }

    .error-message {
      background: #fee;
      color: #c33;
      padding: 12px;
      border-radius: 6px;
      font-size: 14px;
      margin-bottom: 20px;
      display: none;
    }

    .error-message.show {
      display: block;
    }

    .spinner {
      display: inline-block;
      width: 14px;
      height: 14px;
      border: 2px solid #ffffff;
      border-top-color: transparent;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
      margin-right: 8px;
      vertical-align: middle;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }
  </style>
</head>
<body>
  <div class="login-container">
    <div class="login-header">
      <h1>📁 文件管理器</h1>
      <p>请输入密码以继续</p>
    </div>

    <div class="error-message" id="errorMessage"></div>

    <form id="loginForm">
      <div class="form-group">
        <label for="password">密码</label>
        <input type="password" id="password" name="password" required autofocus>
      </div>

      <button type="submit" class="btn-login" id="loginBtn">登录</button>
    </form>
  </div>

  <script>
    const loginForm = document.getElementById('loginForm');
    const passwordInput = document.getElementById('password');
    const loginBtn = document.getElementById('loginBtn');
    const errorMessage = document.getElementById('errorMessage');

    loginForm.addEventListener('submit', async (e) => {
      e.preventDefault();

      const password = passwordInput.value.trim();
      if (!password) {
        showError('请输入密码');
        return;
      }

      // 显示加载状态
      loginBtn.disabled = true;
      loginBtn.innerHTML = '<span class="spinner"></span>登录中...';

      try {
        const response = await fetch('/__filemanager/api/login', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ password }),
        });

        const data = await response.json();

        if (data.success) {
          // 登录成功，重新加载页面
          window.location.reload();
        } else {
          showError(data.error || '登录失败，请重试');
          loginBtn.disabled = false;
          loginBtn.textContent = '登录';
        }
      } catch (error) {
        showError('网络错误，请检查连接');
        loginBtn.disabled = false;
        loginBtn.textContent = '登录';
      }
    });

    function showError(message) {
      errorMessage.textContent = message;
      errorMessage.classList.add('show');
      setTimeout(() => {
        errorMessage.classList.remove('show');
      }, 5000);
    }
  </script>
</body>
</html>`;
}

/**
 * 生成文件管理器主界面
 * @param rootPath 网站根目录路径
 */
export function generateFileManagerPage(rootPath: string): string {
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>文件管理器</title>
  <style>
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }

    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      background: #f5f5f5;
      color: #333;
    }

    .container {
      max-width: 1400px;
      margin: 0 auto;
      padding: 20px;
    }

    /* 顶部导航栏 */
    .header {
      background: white;
      border-radius: 8px;
      padding: 20px;
      margin-bottom: 20px;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
      gap: 15px;
    }

    .breadcrumb {
      display: flex;
      align-items: center;
      gap: 8px;
      flex: 1;
      overflow-x: auto;
    }

    .breadcrumb-item {
      color: #667eea;
      text-decoration: none;
      white-space: nowrap;
      cursor: pointer;
    }

    .breadcrumb-item:hover {
      text-decoration: underline;
    }

    .breadcrumb-separator {
      color: #999;
    }

    .breadcrumb-current {
      color: #333;
      font-weight: 500;
    }

    .actions {
      display: flex;
      gap: 10px;
      flex-wrap: wrap;
    }

    .btn {
      padding: 10px 20px;
      border: none;
      border-radius: 6px;
      font-size: 14px;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
      display: inline-flex;
      align-items: center;
      gap: 6px;
    }

    .btn-primary {
      background: #667eea;
      color: white;
    }

    .btn-primary:hover {
      background: #5568d3;
    }

    .btn-secondary {
      background: #e0e7ff;
      color: #667eea;
    }

    .btn-secondary:hover {
      background: #c7d2fe;
    }

    .btn-danger {
      background: #fee2e2;
      color: #dc2626;
    }

    .btn-danger:hover {
      background: #fecaca;
    }

    /* 文件列表 */
    .file-list {
      background: white;
      border-radius: 8px;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
      overflow: hidden;
    }

    .file-item {
      display: grid;
      grid-template-columns: 30px 40px 2fr 150px 180px 150px;
      align-items: center;
      padding: 12px 20px;
      border-bottom: 1px solid #f0f0f0;
      transition: background 0.2s;
    }

    .file-item:hover {
      background: #f9f9f9;
    }

    .file-item.header {
      background: #f9f9f9;
      font-weight: 600;
      color: #666;
      font-size: 13px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .file-item.header:hover {
      background: #f9f9f9;
    }

    .file-icon {
      font-size: 24px;
    }

    .file-name {
      color: #333;
      text-decoration: none;
      cursor: pointer;
      word-break: break-word;
    }

    .file-name:hover {
      color: #667eea;
    }

    .file-name.directory {
      font-weight: 500;
    }

    .file-size,
    .file-date {
      color: #666;
      font-size: 14px;
    }

    .file-actions {
      display: flex;
      gap: 8px;
    }

    .icon-btn {
      background: none;
      border: none;
      cursor: pointer;
      font-size: 18px;
      padding: 4px;
      border-radius: 4px;
      transition: background 0.2s;
    }

    .icon-btn:hover {
      background: #f0f0f0;
    }

    /* 空状态 */
    .empty-state {
      text-align: center;
      padding: 60px 20px;
      color: #999;
    }

    .empty-state-icon {
      font-size: 48px;
      margin-bottom: 16px;
    }

    /* 加载状态 */
    .loading {
      text-align: center;
      padding: 60px 20px;
      color: #999;
    }

    .spinner {
      display: inline-block;
      width: 40px;
      height: 40px;
      border: 3px solid #f0f0f0;
      border-top-color: #667eea;
      border-radius: 50%;
      animation: spin 1s linear infinite;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    /* 模态框 */
    .modal-overlay {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: rgba(0, 0, 0, 0.5);
      display: none;
      align-items: center;
      justify-content: center;
      z-index: 1000;
    }

    .modal-overlay.show {
      display: flex;
    }

    .modal {
      background: white;
      border-radius: 12px;
      padding: 30px;
      width: 90%;
      max-width: 500px;
      box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    }

    .modal-header {
      font-size: 20px;
      font-weight: 600;
      margin-bottom: 20px;
    }

    .modal-body {
      margin-bottom: 20px;
    }

    .modal-input {
      width: 100%;
      padding: 12px;
      border: 1px solid #ddd;
      border-radius: 6px;
      font-size: 14px;
    }

    .modal-footer {
      display: flex;
      gap: 10px;
      justify-content: flex-end;
    }

    /* Toast 通知 */
    .toast {
      position: fixed;
      bottom: 20px;
      right: 20px;
      background: #333;
      color: white;
      padding: 12px 24px;
      border-radius: 6px;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
      z-index: 1001;
      animation: slideIn 0.3s ease;
    }

    @keyframes slideIn {
      from {
        transform: translateX(100%);
        opacity: 0;
      }
      to {
        transform: translateX(0);
        opacity: 1;
      }
    }

    .toast.success {
      background: #10b981;
    }

    .toast.error {
      background: #ef4444;
    }

    /* 上传区域 */
    .upload-area {
      border: 2px dashed #ddd;
      border-radius: 8px;
      padding: 40px;
      text-align: center;
      cursor: pointer;
      transition: all 0.2s;
      margin-bottom: 20px;
    }

    .upload-area:hover,
    .upload-area.dragover {
      border-color: #667eea;
      background: #f0f4ff;
    }

    .upload-area-icon {
      font-size: 48px;
      margin-bottom: 16px;
    }

    .hidden {
      display: none !important;
    }

    /* 文件复选框 */
    .file-checkbox {
      width: 18px;
      height: 18px;
      cursor: pointer;
    }

    /* 选中计数显示 */
    .selected-count {
      padding: 10px 20px;
      background: #667eea;
      color: white;
      border-radius: 6px;
      font-size: 14px;
      display: none;
      align-items: center;
      gap: 6px;
    }

    .selected-count.show {
      display: inline-flex;
    }

    /* 解压按钮 */
    .extract-btn {
      font-size: 18px;
    }

    @media (max-width: 768px) {
      .file-item {
        grid-template-columns: 30px 40px 1fr 80px;
        gap: 8px;
      }

      .file-size,
      .file-date {
        display: none;
      }

      .header {
        flex-direction: column;
        align-items: stretch;
      }

      .actions {
        width: 100%;
      }

      .btn {
        flex: 1;
      }
    }
  </style>
</head>
<body>
  <div class="container">
    <!-- 顶部导航 -->
    <div class="header">
      <div class="breadcrumb" id="breadcrumb"></div>
      <div class="actions">
        <button class="btn btn-secondary" id="compressBtn">📦 压缩选中项</button>
        <button class="btn btn-danger" id="batchDeleteBtn" disabled>🗑️ 删除选中项</button>
        <div class="selected-count" id="selectedCount">已选择 <span id="selectedNumber">0</span> 项</div>
        <button class="btn btn-primary" id="uploadBtn">📤 上传</button>
        <button class="btn btn-secondary" id="mkdirBtn">📁 新建目录</button>
        <button class="btn btn-danger" id="logoutBtn">🚪 退出</button>
      </div>
    </div>

    <!-- 文件列表 -->
    <div class="file-list">
      <div class="file-item header">
        <div></div>
        <div></div>
        <div>名称</div>
        <div>大小</div>
        <div>修改时间</div>
        <div>操作</div>
      </div>
      <div id="fileList"></div>
    </div>

    <!-- 空状态 -->
    <div class="empty-state hidden" id="emptyState">
      <div class="empty-state-icon">📁</div>
      <p>此目录为空</p>
    </div>

    <!-- 加载状态 -->
    <div class="loading hidden" id="loading">
      <div class="spinner"></div>
    </div>
  </div>

  <!-- 上传模态框 -->
  <div class="modal-overlay" id="uploadModal">
    <div class="modal">
      <div class="modal-header">上传文件</div>
      <div class="modal-body">
        <div class="upload-area" id="uploadArea">
          <div class="upload-area-icon">📤</div>
          <p>点击或拖拽文件到此处上传</p>
          <input type="file" id="fileInput" style="display: none;">
        </div>
        <div id="uploadProgress" class="hidden"></div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="uploadCancelBtn">取消</button>
      </div>
    </div>
  </div>

  <!-- 新建目录模态框 -->
  <div class="modal-overlay" id="mkdirModal">
    <div class="modal">
      <div class="modal-header">新建目录</div>
      <div class="modal-body">
        <input type="text" class="modal-input" id="mkdirInput" placeholder="目录名称">
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="mkdirCancelBtn">取消</button>
        <button class="btn btn-primary" id="mkdirConfirmBtn">创建</button>
      </div>
    </div>
  </div>

  <!-- 重命名模态框 -->
  <div class="modal-overlay" id="renameModal">
    <div class="modal">
      <div class="modal-header">重命名</div>
      <div class="modal-body">
        <input type="text" class="modal-input" id="renameInput" placeholder="新名称">
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="renameCancelBtn">取消</button>
        <button class="btn btn-primary" id="renameConfirmBtn">确定</button>
      </div>
    </div>
  </div>

  <!-- 解压模态框 -->
  <div class="modal-overlay" id="extractModal">
    <div class="modal">
      <div class="modal-header">解压文件</div>
      <div class="modal-body">
        <p>文件：<strong id="extractFileName"></strong></p>
        <label>目标目录：</label>
        <input type="text" class="modal-input" id="extractTargetDir" placeholder="留空解压到当前目录">
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="extractCancelBtn">取消</button>
        <button class="btn btn-primary" id="extractConfirmBtn">解压</button>
      </div>
    </div>
  </div>

  <!-- 压缩模态框 -->
  <div class="modal-overlay" id="compressModal">
    <div class="modal">
      <div class="modal-header">压缩文件</div>
      <div class="modal-body">
        <p>已选择 <strong id="compressFileCount">0</strong> 个文件/目录</p>
        <label>ZIP 文件名：</label>
        <input type="text" class="modal-input" id="compressFileName" placeholder="archive.zip">
        <label style="display: block; margin-top: 10px;">
          <input type="checkbox" id="compressIncludeSrc"> 包含父目录
        </label>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="compressCancelBtn">取消</button>
        <button class="btn btn-primary" id="compressConfirmBtn">压缩</button>
      </div>
    </div>
  </div>

  <script>
    // 网站根目录（从服务器注入）
    const WEB_ROOT = '${rootPath.replace(/\\/g, '/')}';

    // 状态
    let currentPath = WEB_ROOT;
    let csrfToken = null;
    let currentRenamePath = null;
    let currentExtractPath = null;
    let selectedFiles = new Set();

    // 获取当前路径（从服务器获取）
    async function getCurrentPath() {
      // 这里应该从 API 获取初始路径
      return currentPath || '/';
    }

    // 加载目录内容
    async function loadDirectory(path) {
      showLoading(true);

      try {
        const response = await fetch('/__filemanager/api/browse?path=' + encodeURIComponent(path));
        const data = await response.json();

        if (data.success) {
          currentPath = data.data.path;
          renderBreadcrumb(data.data.path, data.data.parentPath);
          renderFileList(data.data.files);
        } else {
          showToast(data.error || '加载失败', 'error');
        }
      } catch (error) {
        showToast('网络错误', 'error');
      } finally {
        showLoading(false);
      }
    }

    // 渲染面包屑
    function renderBreadcrumb(path, parentPath) {
      const breadcrumb = document.getElementById('breadcrumb');
      const parts = path.split(/[\\\\\\/]/).filter(p => p);

      let html = '<span class="breadcrumb-item" data-path="/">🏠 根目录</span>';

      let currentPath = '';
      for (const part of parts) {
        currentPath = currentPath ? currentPath + '/' + part : part;
        html += '<span class="breadcrumb-separator">/</span>';
        html += '<span class="breadcrumb-item" data-path="' + currentPath + '">' + part + '</span>';
      }

      breadcrumb.innerHTML = html;

      // 添加点击事件
      breadcrumb.querySelectorAll('.breadcrumb-item').forEach(item => {
        item.addEventListener('click', () => {
          loadDirectory(item.dataset.path);
        });
      });
    }

    // 渲染文件列表
    function renderFileList(files) {
      const fileList = document.getElementById('fileList');
      const emptyState = document.getElementById('emptyState');

      // 清空选中项
      selectedFiles.clear();
      updateSelectedCount();

      if (files.length === 0) {
        fileList.innerHTML = '';
        emptyState.classList.remove('hidden');
        return;
      }

      emptyState.classList.add('hidden');

      let html = '';
      for (const file of files) {
        const icon = file.isDirectory ? '📁' : getFileIcon(file.name);
        const size = file.isDirectory ? '-' : formatFileSize(file.size);
        const date = formatDateTime(new Date(file.modifiedTime));
        const filePath = currentPath + '/' + file.name;
        const isArchive = isArchiveFile(file.name);

        html += '<div class="file-item">';
        html += '<div><input type="checkbox" class="file-checkbox" data-path="' + filePath + '"></div>';
        html += '<div class="file-icon">' + icon + '</div>';
        html += '<div class="file-name ' + (file.isDirectory ? 'directory' : '') + '" data-path="' + filePath + '">' + file.name + '</div>';
        html += '<div class="file-size">' + size + '</div>';
        html += '<div class="file-date">' + date + '</div>';
        html += '<div class="file-actions">';
        html += '<button class="icon-btn download-btn" title="下载" data-path="' + filePath + '">📥</button>';

        // 如果是压缩文件，添加解压按钮
        if (isArchive) {
          html += '<button class="icon-btn extract-btn" title="解压" data-path="' + filePath + '" data-name="' + file.name + '">📦</button>';
        }

        // 重命名按钮（仅文件）
        if (!file.isDirectory) {
          html += '<button class="icon-btn rename-btn" title="重命名" data-path="' + filePath + '" data-name="' + file.name + '">✏️</button>';
        }

        // 删除按钮（文件和目录都可以删除）
        html += '<button class="icon-btn delete-btn" title="删除" data-path="' + filePath + '" data-is-directory="' + file.isDirectory + '">🗑️</button>';

        html += '</div>';
        html += '</div>';
      }

      fileList.innerHTML = html;

      // 添加事件监听
      fileList.querySelectorAll('.file-name.directory').forEach(item => {
        item.addEventListener('click', () => {
          loadDirectory(item.dataset.path);
        });
      });

      fileList.querySelectorAll('.file-checkbox').forEach(checkbox => {
        checkbox.addEventListener('change', () => {
          if (checkbox.checked) {
            selectedFiles.add(checkbox.dataset.path);
          } else {
            selectedFiles.delete(checkbox.dataset.path);
          }
          updateSelectedCount();
        });
      });

      fileList.querySelectorAll('.download-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          downloadFile(btn.dataset.path);
        });
      });

      fileList.querySelectorAll('.extract-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          showExtractModal(btn.dataset.path, btn.dataset.name);
        });
      });

      fileList.querySelectorAll('.rename-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          showRenameModal(btn.dataset.path, btn.dataset.name);
        });
      });

      fileList.querySelectorAll('.delete-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          const isDirectory = btn.dataset.isDirectory === 'true';
          deleteFile(btn.dataset.path, isDirectory);
        });
      });
    }

    // 获取文件图标
    function getFileIcon(filename) {
      const ext = filename.split('.').pop().toLowerCase();
      const icons = {
        'jpg': '🖼️', 'jpeg': '🖼️', 'png': '🖼️', 'gif': '🖼️', 'svg': '🖼️',
        'mp4': '🎬', 'avi': '🎬', 'mov': '🎬', 'wmv': '🎬',
        'mp3': '🎵', 'wav': '🎵', 'flac': '🎵',
        'pdf': '📄', 'doc': '📄', 'docx': '📄', 'txt': '📄',
        'zip': '📦', 'rar': '📦', '7z': '📦', 'tar': '📦', 'gz': '📦',
        'js': '📜', 'ts': '📜', 'html': '📜', 'css': '📜', 'json': '📜',
      };
      return icons[ext] || '📄';
    }

    // 格式化文件大小
    function formatFileSize(bytes) {
      if (bytes === 0) return '0 B';
      const k = 1024;
      const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    // 格式化日期时间
    function formatDateTime(date) {
      const year = date.getFullYear();
      const month = String(date.getMonth() + 1).padStart(2, '0');
      const day = String(date.getDate()).padStart(2, '0');
      const hours = String(date.getHours()).padStart(2, '0');
      const minutes = String(date.getMinutes()).padStart(2, '0');
      return year + '-' + month + '-' + day + ' ' + hours + ':' + minutes;
    }

    // 显示/隐藏加载状态
    function showLoading(show) {
      document.getElementById('loading').classList.toggle('hidden', !show);
    }

    // 显示 Toast 通知
    function showToast(message, type = 'success') {
      const toast = document.createElement('div');
      toast.className = 'toast ' + type;
      toast.textContent = message;
      document.body.appendChild(toast);

      setTimeout(() => {
        toast.remove();
      }, 3000);
    }

    // 下载文件
    function downloadFile(path) {
      window.open('/__filemanager/api/download?path=' + encodeURIComponent(path), '_blank');
    }

    // 删除文件或目录
    async function deleteFile(path, isDirectory) {
      var itemType = isDirectory ? '目录' : '文件';
      if (!confirm('确定要删除' + itemType + '吗？此操作不可恢复！')) {
        return;
      }

      try {
        const response = await fetch('/__filemanager/api/delete', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ path }),
        });

        const data = await response.json();

        if (data.success) {
          showToast('删除成功');
          loadDirectory(currentPath);
        } else {
          showToast(data.error || '删除失败', 'error');
        }
      } catch (error) {
        showToast('网络错误', 'error');
      }
    }

    // 批量删除文件
    async function batchDeleteFiles() {
      var count = selectedFiles.size;
      if (count === 0) {
        return;
      }

      if (!confirm('确定要删除选中的 ' + count + ' 项吗？此操作不可恢复！')) {
        return;
      }

      var successCount = 0;
      var failCount = 0;

      for (const path of selectedFiles) {
        try {
          const response = await fetch('/__filemanager/api/delete', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({ path }),
          });

          const data = await response.json();

          if (data.success) {
            successCount++;
          } else {
            failCount++;
            console.error('删除失败: ' + path + ' - ' + data.error);
          }
        } catch (error) {
          failCount++;
          console.error('删除错误: ' + path, error);
        }
      }

      // 清空选中项
      selectedFiles.clear();
      updateSelectedCount();

      // 显示结果
      if (failCount === 0) {
        showToast('成功删除 ' + successCount + ' 项');
      } else {
        showToast('删除完成：成功 ' + successCount + ' 项，失败 ' + failCount + ' 项', 'error');
      }

      // 刷新目录
      loadDirectory(currentPath);
    }

    // 显示重命名模态框
    function showRenameModal(path, name) {
      currentRenamePath = path;
      document.getElementById('renameInput').value = name;
      document.getElementById('renameModal').classList.add('show');
    }

    // 隐藏重命名模态框
    function hideRenameModal() {
      document.getElementById('renameModal').classList.remove('show');
      currentRenamePath = null;
    }

    // 执行重命名
    async function doRename() {
      const newName = document.getElementById('renameInput').value.trim();
      if (!newName) {
        showToast('请输入新名称', 'error');
        return;
      }

      try {
        const response = await fetch('/__filemanager/api/rename', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ oldPath: currentRenamePath, newName }),
        });

        const data = await response.json();

        if (data.success) {
          showToast('重命名成功');
          hideRenameModal();
          loadDirectory(currentPath);
        } else {
          showToast(data.error || '重命名失败', 'error');
        }
      } catch (error) {
        showToast('网络错误', 'error');
      }
    }

    // 上传文件
    async function uploadFile(file) {
      const formData = new FormData();
      formData.append('file', file);
      formData.append('path', currentPath);

      try {
        const response = await fetch('/__filemanager/api/upload?path=' + encodeURIComponent(currentPath), {
          method: 'POST',
          body: formData,
        });

        const data = await response.json();

        if (data.success) {
          showToast('上传成功');
          loadDirectory(currentPath);
        } else {
          showToast(data.error || '上传失败', 'error');
        }
      } catch (error) {
        showToast('网络错误', 'error');
      }
    }

    // 创建目录
    async function createDirectory(name) {
      try {
        const response = await fetch('/__filemanager/api/mkdir', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ parentPath: currentPath, dirName: name }),
        });

        const data = await response.json();

        if (data.success) {
          showToast('目录创建成功');
          loadDirectory(currentPath);
        } else {
          showToast(data.error || '创建失败', 'error');
        }
      } catch (error) {
        showToast('网络错误', 'error');
      }
    }

    // 登出
    async function logout() {
      if (!confirm('确定要退出吗？')) {
        return;
      }

      try {
        await fetch('/__filemanager/api/logout', { method: 'POST' });
        window.location.reload();
      } catch (error) {
        showToast('网络错误', 'error');
      }
    }

    // 检查是否为压缩文件
    function isArchiveFile(filename) {
      const lowerName = filename.toLowerCase();
      return lowerName.endsWith('.zip') ||
             lowerName.endsWith('.tar') ||
             lowerName.endsWith('.tar.gz') ||
             lowerName.endsWith('.tgz');
    }

    // 更新选中计数
    function updateSelectedCount() {
      const count = selectedFiles.size;
      document.getElementById('selectedNumber').textContent = count;
      document.getElementById('selectedCount').classList.toggle('show', count > 0);
      document.getElementById('compressBtn').disabled = count === 0;
      document.getElementById('batchDeleteBtn').disabled = count === 0;
    }

    // 显示解压模态框
    function showExtractModal(path, name) {
      currentExtractPath = path;
      document.getElementById('extractFileName').textContent = name;
      document.getElementById('extractTargetDir').value = currentPath;
      document.getElementById('extractModal').classList.add('show');
    }

    // 隐藏解压模态框
    function hideExtractModal() {
      document.getElementById('extractModal').classList.remove('show');
      currentExtractPath = null;
    }

    // 执行解压
    async function doExtract() {
      const targetDir = document.getElementById('extractTargetDir').value.trim() || currentPath;

      try {
        const response = await fetch('/__filemanager/api/extract', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            archivePath: currentExtractPath,
            targetDir: targetDir,
          }),
        });

        const data = await response.json();

        if (data.success) {
          showToast('解压成功');
          hideExtractModal();
          loadDirectory(currentPath);
        } else {
          showToast(data.error || '解压失败', 'error');
        }
      } catch (error) {
        showToast('网络错误', 'error');
      }
    }

    // 显示压缩模态框
    function showCompressModal() {
      const count = selectedFiles.size;
      document.getElementById('compressFileCount').textContent = count;
      document.getElementById('compressFileName').value = 'archive.zip';
      document.getElementById('compressIncludeSrc').checked = false;
      document.getElementById('compressModal').classList.add('show');
    }

    // 隐藏压缩模态框
    function hideCompressModal() {
      document.getElementById('compressModal').classList.remove('show');
    }

    // 执行压缩
    async function doCompress() {
      const fileName = document.getElementById('compressFileName').value.trim();
      const includeSrc = document.getElementById('compressIncludeSrc').checked;

      if (!fileName) {
        showToast('请输入文件名', 'error');
        return;
      }

      if (!fileName.toLowerCase().endsWith('.zip')) {
        showToast('文件名必须以 .zip 结尾', 'error');
        return;
      }

      const targetPath = currentPath + '/' + fileName;

      try {
        const response = await fetch('/__filemanager/api/compress', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            sourcePaths: Array.from(selectedFiles),
            targetPath: targetPath,
            includeSrc: includeSrc,
          }),
        });

        const data = await response.json();

        if (data.success) {
          showToast('压缩成功');
          hideCompressModal();
          loadDirectory(currentPath);
        } else {
          showToast(data.error || '压缩失败', 'error');
        }
      } catch (error) {
        showToast('网络错误', 'error');
      }
    }

    // 事件监听
    document.getElementById('uploadBtn').addEventListener('click', () => {
      document.getElementById('uploadModal').classList.add('show');
    });

    document.getElementById('uploadCancelBtn').addEventListener('click', () => {
      document.getElementById('uploadModal').classList.remove('show');
    });

    document.getElementById('uploadArea').addEventListener('click', () => {
      document.getElementById('fileInput').click();
    });

    document.getElementById('fileInput').addEventListener('change', (e) => {
      const file = e.target.files[0];
      if (file) {
        uploadFile(file);
        document.getElementById('uploadModal').classList.remove('show');
      }
    });

    const uploadArea = document.getElementById('uploadArea');
    uploadArea.addEventListener('dragover', (e) => {
      e.preventDefault();
      uploadArea.classList.add('dragover');
    });

    uploadArea.addEventListener('dragleave', () => {
      uploadArea.classList.remove('dragover');
    });

    uploadArea.addEventListener('drop', (e) => {
      e.preventDefault();
      uploadArea.classList.remove('dragover');
      const file = e.dataTransfer.files[0];
      if (file) {
        uploadFile(file);
        document.getElementById('uploadModal').classList.remove('show');
      }
    });

    document.getElementById('mkdirBtn').addEventListener('click', () => {
      document.getElementById('mkdirInput').value = '';
      document.getElementById('mkdirModal').classList.add('show');
    });

    document.getElementById('mkdirCancelBtn').addEventListener('click', () => {
      document.getElementById('mkdirModal').classList.remove('show');
    });

    document.getElementById('mkdirConfirmBtn').addEventListener('click', () => {
      const name = document.getElementById('mkdirInput').value.trim();
      if (name) {
        createDirectory(name);
        document.getElementById('mkdirModal').classList.remove('show');
      }
    });

    document.getElementById('renameCancelBtn').addEventListener('click', hideRenameModal);
    document.getElementById('renameConfirmBtn').addEventListener('click', doRename);
    document.getElementById('logoutBtn').addEventListener('click', logout);

    // 解压模态框事件
    document.getElementById('extractCancelBtn').addEventListener('click', hideExtractModal);
    document.getElementById('extractConfirmBtn').addEventListener('click', doExtract);

    // 压缩模态框事件
    document.getElementById('compressBtn').addEventListener('click', showCompressModal);
    document.getElementById('compressCancelBtn').addEventListener('click', hideCompressModal);
    document.getElementById('compressConfirmBtn').addEventListener('click', doCompress);

    // 批量删除按钮事件
    document.getElementById('batchDeleteBtn').addEventListener('click', batchDeleteFiles);

    // 初始化：加载网站根目录
    loadDirectory(WEB_ROOT);
  </script>
</body>
</html>`;
}

/**
 * 生成错误页面
 */
export function generateErrorPage(message: string): string {
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>错误</title>
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
      background: #f5f5f5;
      margin: 0;
      padding: 20px;
    }
    .error-container {
      background: white;
      border-radius: 12px;
      padding: 40px;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
      text-align: center;
      max-width: 500px;
    }
    h1 {
      color: #dc2626;
      margin-bottom: 20px;
    }
    p {
      color: #666;
      line-height: 1.6;
    }
  </style>
</head>
<body>
  <div class="error-container">
    <h1>⚠️ 错误</h1>
    <p>${message}</p>
  </div>
</body>
</html>`;
}
