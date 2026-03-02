/**
 * File manager frontend interface template
 * Generates login page and main interface
 */

import { formatFileSize, formatDateTime } from "./config.ts";

/**
 * Generate login page
 */
export function generateLoginPage(): string {
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>File Manager - Login</title>
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
      <h1>File Manager</h1>
      <p>Please enter password to continue</p>
    </div>

    <div class="error-message" id="errorMessage"></div>

    <form id="loginForm">
      <div class="form-group">
        <label for="password">Password</label>
        <input type="password" id="password" name="password" required autofocus>
      </div>

      <button type="submit" class="btn-login" id="loginBtn">Login</button>
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
        showError('Please enter password');
        return;
      }

      // Show loading state
      loginBtn.disabled = true;
      loginBtn.innerHTML = '<span class="spinner"></span>Logging in...';

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
          // Login successful, reload page
          window.location.reload();
        } else {
          showError(data.error || 'Login failed, please try again');
          loginBtn.disabled = false;
          loginBtn.textContent = 'Login';
        }
      } catch (error) {
        showError('Network error, please check connection');
        loginBtn.disabled = false;
        loginBtn.textContent = 'Login';
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
 * Generate file manager main interface
 * @param rootPath Web root directory path
 */
export function generateFileManagerPage(rootPath: string): string {
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>File Manager</title>
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

    /* Top navigation bar */
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

    /* File list */
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

    /* Empty state */
    .empty-state {
      text-align: center;
      padding: 60px 20px;
      color: #999;
    }

    .empty-state-icon {
      font-size: 48px;
      margin-bottom: 16px;
    }

    /* Loading state */
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

    /* Modal */
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

    /* Toast notification */
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

    /* Upload area */
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

    /* File checkbox */
    .file-checkbox {
      width: 18px;
      height: 18px;
      cursor: pointer;
    }

    /* Selected count display */
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

    /* Extract button */
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
    <!-- Top navigation -->
    <div class="header">
      <div class="breadcrumb" id="breadcrumb"></div>
      <div class="actions">
        <button class="btn btn-secondary" id="batchMoveBtn" disabled>Move Selected</button>
        <button class="btn btn-secondary" id="compressBtn">Compress Selected</button>
        <button class="btn btn-danger" id="batchDeleteBtn" disabled>Delete Selected</button>
        <div class="selected-count" id="selectedCount">Selected <span id="selectedNumber">0</span> items</div>
        <button class="btn btn-primary" id="uploadBtn">Upload</button>
        <button class="btn btn-secondary" id="mkdirBtn">New Directory</button>
        <button class="btn btn-danger" id="logoutBtn">Exit</button>
      </div>
    </div>

    <!-- File list -->
    <div class="file-list">
      <div class="file-item header">
        <div></div>
        <div></div>
        <div>Name</div>
        <div>Size</div>
        <div>Modified</div>
        <div>Actions</div>
      </div>
      <div id="fileList"></div>
    </div>

    <!-- Empty state -->
    <div class="empty-state hidden" id="emptyState">
      <div class="empty-state-icon"></div>
      <p>This directory is empty</p>
    </div>

    <!-- Loading state -->
    <div class="loading hidden" id="loading">
      <div class="spinner"></div>
    </div>
  </div>

  <!-- Upload modal -->
  <div class="modal-overlay" id="uploadModal">
    <div class="modal">
      <div class="modal-header">Upload File</div>
      <div class="modal-body">
        <div class="upload-area" id="uploadArea">
          <div class="upload-area-icon"></div>
          <p>Click or drag files here to upload</p>
          <input type="file" id="fileInput" style="display: none;">
        </div>
        <div id="uploadProgress" class="hidden"></div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="uploadCancelBtn">Cancel</button>
      </div>
    </div>
  </div>

  <!-- New directory modal -->
  <div class="modal-overlay" id="mkdirModal">
    <div class="modal">
      <div class="modal-header">New Directory</div>
      <div class="modal-body">
        <input type="text" class="modal-input" id="mkdirInput" placeholder="Directory name">
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="mkdirCancelBtn">Cancel</button>
        <button class="btn btn-primary" id="mkdirConfirmBtn">Create</button>
      </div>
    </div>
  </div>

  <!-- Rename modal -->
  <div class="modal-overlay" id="renameModal">
    <div class="modal">
      <div class="modal-header">Rename</div>
      <div class="modal-body">
        <input type="text" class="modal-input" id="renameInput" placeholder="New name">
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="renameCancelBtn">Cancel</button>
        <button class="btn btn-primary" id="renameConfirmBtn">Confirm</button>
      </div>
    </div>
  </div>

  <!-- Extract modal -->
  <div class="modal-overlay" id="extractModal">
    <div class="modal">
      <div class="modal-header">Extract File</div>
      <div class="modal-body">
        <p>File: <strong id="extractFileName"></strong></p>
        <label>Target directory:</label>
        <input type="text" class="modal-input" id="extractTargetDir" placeholder="Default is archive directory">
        <p style="font-size: 12px; color: #666; margin-top: 10px;">
          Leave empty to extract to archive directory
        </p>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="extractCancelBtn">Cancel</button>
        <button class="btn btn-primary" id="extractConfirmBtn">Extract</button>
      </div>
    </div>
  </div>

  <!-- Compress modal -->
  <div class="modal-overlay" id="compressModal">
    <div class="modal">
      <div class="modal-header">Compress File</div>
      <div class="modal-body">
        <p>Selected <strong id="compressFileCount">0</strong> files/directories</p>
        <label>ZIP filename:</label>
        <input type="text" class="modal-input" id="compressFileName" placeholder="archive.zip">
        <label style="display: block; margin-top: 10px;">
          <input type="checkbox" id="compressIncludeSrc"> Include parent directory
        </label>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="compressCancelBtn">Cancel</button>
        <button class="btn btn-primary" id="compressConfirmBtn">Compress</button>
      </div>
    </div>
  </div>

  <!-- Batch move modal -->
  <div class="modal-overlay" id="moveModal">
    <div class="modal">
      <div class="modal-header">Move File</div>
      <div class="modal-body">
        <p>Selected <strong id="moveFileCount">0</strong> files/directories</p>
        <label>Target directory:</label>
        <input type="text" class="modal-input" id="moveTargetDir" placeholder="Enter target directory path">
        <p style="font-size: 12px; color: #666; margin-top: 10px;">
          Tip: Target directory must exist. You can use relative path (e.g., ../subdir) or absolute path.
        </p>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" id="moveCancelBtn">Cancel</button>
        <button class="btn btn-primary" id="moveConfirmBtn">Move</button>
      </div>
    </div>
  </div>

  <script>
    // Web root directory (injected from server)
    const WEB_ROOT = '${rootPath.replace(/\\/g, '/')}';

    // State
    let currentPath = WEB_ROOT;
    let csrfToken = null;
    let currentRenamePath = null;
    let currentExtractPath = null;
    let selectedFiles = new Set();

    // Get current path (from server)
    async function getCurrentPath() {
      // Should get initial path from API here
      return currentPath || '/';
    }

    // Load directory contents
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
          showToast(data.error || 'Failed to load', 'error');
        }
      } catch (error) {
        showToast('Network error', 'error');
      } finally {
        showLoading(false);
      }
    }

    // Render breadcrumb
    function renderBreadcrumb(path, parentPath) {
      const breadcrumb = document.getElementById('breadcrumb');
      const parts = path.split(/[\\\\\\/]/).filter(p => p);

      let html = '<span class="breadcrumb-item" data-path="/">Root</span>';

      let currentPath = '';
      for (const part of parts) {
        currentPath = currentPath ? currentPath + '/' + part : part;
        html += '<span class="breadcrumb-separator">/</span>';
        html += '<span class="breadcrumb-item" data-path="' + currentPath + '">' + part + '</span>';
      }

      breadcrumb.innerHTML = html;

      // Add click event
      breadcrumb.querySelectorAll('.breadcrumb-item').forEach(item => {
        item.addEventListener('click', () => {
          loadDirectory(item.dataset.path);
        });
      });
    }

    // Render file list
    function renderFileList(files) {
      const fileList = document.getElementById('fileList');
      const emptyState = document.getElementById('emptyState');

      // Clear selected items
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
        html += '<button class="icon-btn download-btn" title="Download" data-path="' + filePath + '"></button>';

        // If archive file, add extract button
        if (isArchive) {
          html += '<button class="icon-btn extract-btn" title="Extract" data-path="' + filePath + '" data-name="' + file.name + '"></button>';
        }

        // Rename button (files only)
        if (!file.isDirectory) {
          html += '<button class="icon-btn rename-btn" title="Rename" data-path="' + filePath + '" data-name="' + file.name + '"></button>';
        }

        // Delete button (both files and directories can be deleted)
        html += '<button class="icon-btn delete-btn" title="Delete" data-path="' + filePath + '" data-is-directory="' + file.isDirectory + '"></button>';

        html += '</div>';
        html += '</div>';
      }

      fileList.innerHTML = html;

      // Add event listeners
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

    // Get file icon
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

    // Format file size
    function formatFileSize(bytes) {
      if (bytes === 0) return '0 B';
      const k = 1024;
      const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    // Format datetime
    function formatDateTime(date) {
      const year = date.getFullYear();
      const month = String(date.getMonth() + 1).padStart(2, '0');
      const day = String(date.getDate()).padStart(2, '0');
      const hours = String(date.getHours()).padStart(2, '0');
      const minutes = String(date.getMinutes()).padStart(2, '0');
      return year + '-' + month + '-' + day + ' ' + hours + ':' + minutes;
    }

    // Show/hide loading state
    function showLoading(show) {
      document.getElementById('loading').classList.toggle('hidden', !show);
    }

    // Show Toast notification
    function showToast(message, type = 'success') {
      const toast = document.createElement('div');
      toast.className = 'toast ' + type;
      toast.textContent = message;
      document.body.appendChild(toast);

      setTimeout(() => {
        toast.remove();
      }, 3000);
    }

    // Download file
    function downloadFile(path) {
      window.open('/__filemanager/api/download?path=' + encodeURIComponent(path), '_blank');
    }

    // Delete file or directory
    async function deleteFile(path, isDirectory) {
      var itemType = isDirectory ? 'directory' : 'file';
      if (!confirm('Are you sure you want to delete this ' + itemType + '? This action cannot be undone!')) {
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
          showToast('Deleted successfully');
          loadDirectory(currentPath);
        } else {
          showToast(data.error || 'Delete failed', 'error');
        }
      } catch (error) {
        showToast('Network error', 'error');
      }
    }

    // Batch delete files
    async function batchDeleteFiles() {
      var count = selectedFiles.size;
      if (count === 0) {
        return;
      }

      if (!confirm('Are you sure you want to delete ' + count + ' selected items? This action cannot be undone!')) {
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
            console.error('Delete failed: ' + path + ' - ' + data.error);
          }
        } catch (error) {
          failCount++;
          console.error('Delete error: ' + path, error);
        }
      }

      // Clear selected items
      selectedFiles.clear();
      updateSelectedCount();

      // Show result
      if (failCount === 0) {
        showToast('Successfully deleted ' + successCount + ' items');
      } else {
        showToast('Delete completed: ' + successCount + ' succeeded, ' + failCount + ' failed', 'error');
      }

      // Refresh directory
      loadDirectory(currentPath);
    }

    // Show batch move modal
    function showMoveModal() {
      var count = selectedFiles.size;
      if (count === 0) {
        return;
      }

      document.getElementById('moveFileCount').textContent = count;
      document.getElementById('moveTargetDir').value = currentPath;
      document.getElementById('moveModal').classList.add('show');
    }

    // Hide batch move modal
    function hideMoveModal() {
      document.getElementById('moveModal').classList.remove('show');
    }

    // Execute batch move
    async function doBatchMove() {
      var targetDir = document.getElementById('moveTargetDir').value.trim();
      if (!targetDir) {
        showToast('Please enter target directory', 'error');
        return;
      }

      // Parse relative path
      if (targetDir.startsWith('.')) {
        var currentParts = currentPath.split('/');
        var targetParts = targetDir.split('/');

        for (var i = 0; i < targetParts.length; i++) {
          var part = targetParts[i];
          if (part === '..') {
            currentParts.pop();
          } else if (part !== '.') {
            currentParts.push(part);
          }
        }

        targetDir = currentParts.join('/');
      }

      try {
        const response = await fetch('/__filemanager/api/batch-move', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            sourcePaths: Array.from(selectedFiles),
            targetDir: targetDir,
          }),
        });

        const data = await response.json();

        if (data.success) {
          hideMoveModal();

          // Clear selected items
          selectedFiles.clear();
          updateSelectedCount();

          // Show result
          if (data.data.failedCount === 0) {
            showToast('Successfully moved ' + data.data.successCount + ' items');
          } else {
            showToast('Move completed: ' + data.data.successCount + ' succeeded, ' + data.data.failedCount + ' failed', 'error');
          }

          // Refresh directory
          loadDirectory(currentPath);
        } else {
          showToast(data.error || 'Move failed', 'error');
        }
      } catch (error) {
        showToast('Network error', 'error');
      }
    }

    // Show rename modal
    function showRenameModal(path, name) {
      currentRenamePath = path;
      document.getElementById('renameInput').value = name;
      document.getElementById('renameModal').classList.add('show');
    }

    // Hide rename modal
    function hideRenameModal() {
      document.getElementById('renameModal').classList.remove('show');
      currentRenamePath = null;
    }

    // Execute rename
    async function doRename() {
      const newName = document.getElementById('renameInput').value.trim();
      if (!newName) {
        showToast('Please enter new name', 'error');
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
          showToast('Rename successful');
          hideRenameModal();
          loadDirectory(currentPath);
        } else {
          showToast(data.error || 'Rename failed', 'error');
        }
      } catch (error) {
        showToast('Network error', 'error');
      }
    }

    // Upload file
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
          showToast('Upload successful');
          loadDirectory(currentPath);
        } else {
          showToast(data.error || 'Upload failed', 'error');
        }
      } catch (error) {
        showToast('Network error', 'error');
      }
    }

    // Create directory
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
          showToast('Directory created successfully');
          loadDirectory(currentPath);
        } else {
          showToast(data.error || 'Creation failed', 'error');
        }
      } catch (error) {
        showToast('Network error', 'error');
      }
    }

    // Logout
    async function logout() {
      if (!confirm('Are you sure you want to quit?')) {
        return;
      }

      try {
        await fetch('/__filemanager/api/logout', { method: 'POST' });
        window.location.reload();
      } catch (error) {
        showToast('Network error', 'error');
      }
    }

    // Check if it's an archive file
    function isArchiveFile(filename) {
      const lowerName = filename.toLowerCase();
      return lowerName.endsWith('.zip') ||
             lowerName.endsWith('.tar') ||
             lowerName.endsWith('.tar.gz') ||
             lowerName.endsWith('.tgz');
    }

    // Update selected count
    function updateSelectedCount() {
      const count = selectedFiles.size;
      document.getElementById('selectedNumber').textContent = count;
      document.getElementById('selectedCount').classList.toggle('show', count > 0);
      document.getElementById('batchMoveBtn').disabled = count === 0;
      document.getElementById('compressBtn').disabled = count === 0;
      document.getElementById('batchDeleteBtn').disabled = count === 0;
    }

    // Show extract modal
    function showExtractModal(path, name) {
      currentExtractPath = path;
      document.getElementById('extractFileName').textContent = name;

      // Default extract to the directory where the archive is located (not the current browsing directory)
      var archiveDir = path.substring(0, path.lastIndexOf('/'));
      if (!archiveDir) {
        archiveDir = '/';
      }
      document.getElementById('extractTargetDir').value = archiveDir;

      document.getElementById('extractModal').classList.add('show');
    }

    // Hide extract modal
    function hideExtractModal() {
      document.getElementById('extractModal').classList.remove('show');
      currentExtractPath = null;
    }

    // Execute extract
    async function doExtract() {
      var targetDir = document.getElementById('extractTargetDir').value.trim();

      // If user doesn't specify target directory (empty string), don't pass targetDir parameter
      // Backend will use the archive directory by default
      var requestBody = {
        archivePath: currentExtractPath
      };

      if (targetDir) {
        requestBody.targetDir = targetDir;
      }

      try {
        const response = await fetch('/__filemanager/api/extract', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(requestBody),
        });

        const data = await response.json();

        if (data.success) {
          showToast('Extraction successful');
          hideExtractModal();

          // Refresh directory display
          loadDirectory(currentPath);
        } else {
          showToast(data.error || 'Extraction failed', 'error');
        }
      } catch (error) {
        showToast('Network error', 'error');
      }
    }

    // Show compress modal
    function showCompressModal() {
      const count = selectedFiles.size;
      document.getElementById('compressFileCount').textContent = count;
      document.getElementById('compressFileName').value = 'archive.zip';
      document.getElementById('compressIncludeSrc').checked = false;
      document.getElementById('compressModal').classList.add('show');
    }

    // Hide compress modal
    function hideCompressModal() {
      document.getElementById('compressModal').classList.remove('show');
    }

    // Execute compress
    async function doCompress() {
      const fileName = document.getElementById('compressFileName').value.trim();
      const includeSrc = document.getElementById('compressIncludeSrc').checked;

      if (!fileName) {
        showToast('Please enter file name', 'error');
        return;
      }

      if (!fileName.toLowerCase().endsWith('.zip')) {
        showToast('File name must end with .zip', 'error');
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
          showToast('Compression successful');
          hideCompressModal();
          loadDirectory(currentPath);
        } else {
          showToast(data.error || 'Compression failed', 'error');
        }
      } catch (error) {
        showToast('Network error', 'error');
      }
    }

    // Event listeners
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

    // Extract modal events
    document.getElementById('extractCancelBtn').addEventListener('click', hideExtractModal);
    document.getElementById('extractConfirmBtn').addEventListener('click', doExtract);

    // Compress modal events
    document.getElementById('compressBtn').addEventListener('click', showCompressModal);
    document.getElementById('compressCancelBtn').addEventListener('click', hideCompressModal);
    document.getElementById('compressConfirmBtn').addEventListener('click', doCompress);

    // Batch delete button events
    document.getElementById('batchDeleteBtn').addEventListener('click', batchDeleteFiles);

    // Batch move modal events
    document.getElementById('batchMoveBtn').addEventListener('click', showMoveModal);
    document.getElementById('moveCancelBtn').addEventListener('click', hideMoveModal);
    document.getElementById('moveConfirmBtn').addEventListener('click', doBatchMove);

    // Initialize: load web root directory
    loadDirectory(WEB_ROOT);
  </script>
</body>
</html>`;
}

/**
 * Generate error page
 */
export function generateErrorPage(message: string): string {
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Error</title>
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
    <h1>⚠️ Error</h1>
    <p>${message}</p>
  </div>
</body>
</html>`;
}
