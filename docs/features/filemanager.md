# TSP File Manager

TSP's built-in web file manager, providing online file browsing, upload, download, rename, delete, and more.

## Features

- **Secure Authentication**: Password-based authentication mechanism
- **Path Control**: Configurable whether to allow access outside website root
- **File Operations**: Upload, download, delete, rename, create directory, batch move
- **Extract/Compress**: Support for extracting ZIP, TAR, TAR.GZ files, compressing files/directories to ZIP
- **Batch Operations**: Batch delete, batch move, batch compress
- **Permission Control**: Fine-grained operation permission configuration
- **Security Protection**: Path validation, file type restrictions, size limits, ZIP bomb protection
- **User-friendly Interface**: Modern web interface with drag-and-drop upload support

## Quick Start

### 1. Enable File Manager

Add `fileManager` config section in the config file:

**config.jsonc**:
```jsonc
{
  "root": "./www",
  "port": 9000,
  "fileManager": {
    "enabled": true,
    "password": "your-secure-password"
  }
}
```

### 2. Access File Manager

After starting the server, open in browser:

```
http://localhost:9000/__filemanager
```

Default path is `/__filemanager`, can be modified in config.

### 3. Login

Enter the configured password to log in.

**Important**: After modifying the password in the config file, no server restart is needed. Refresh the browser and log in with the new password.

## Configuration Options

Complete configuration options:

```jsonc
{
  "fileManager": {
    // Required: Whether to enable
    "enabled": true,

    // Optional: Access path (default: "/__filemanager")
    "path": "/__filemanager",

    // Required: Access password (at least 6 characters)
    "password": "your-secure-password",

    // Optional: Allow outside root (default: false)
    "allowOutsideRoot": false,

    // Optional: Allowed path whitelist
    "allowedPaths": ["./www", "./uploads"],

    // Optional: Denied path blacklist (default: [".git", ".deno", "node_modules", ".cache"])
    "deniedPaths": [".git", ".deno", "node_modules", ".cache"],

    // Optional: Max upload size (default: 100MB)
    "maxUploadSize": 104857600,

    // Optional: Allowed upload extensions (empty list means allow all)
    "allowedExtensions": [".jpg", ".png", ".gif", ".pdf", ".doc", ".docx"],

    // Optional: Denied upload extensions (default: [".exe", ".sh", ".bat", ".cmd", ".scr", ".pif"])
    "deniedExtensions": [".exe", ".sh", ".bat"],

    // Optional: Allow delete (default: true)
    "allowDelete": true,

    // Optional: Allow rename (default: true)
    "allowRename": true,

    // Optional: Allow create directory (default: true)
    "allowMkdir": true,

    // Optional: Allow move (default: true)
    "allowMove": true,

    // Optional: Allow extract (default: true)
    "allowExtract": true,

    // Optional: Allow compress (default: true)
    "allowCompress": true,

    // Optional: Allowed archive formats (default: ["tar", "tgz"], zip is disabled due to binary compatibility)
    "allowedArchiveExtensions": ["tar", "tgz"],

    // Optional: Max extract size (default: 1GB)
    "maxExtractSize": 1073741824,

    // Optional: Max compress size (default: 500MB)
    "maxCompressSize": 524288000,

    // Optional: ZIP bomb protection - max file count (default: 10000)
    "maxExtractFileCount": 10000
  }
}
```

### Configuration Reference

#### enabled
- Type: `boolean`
- Default: `false`
- Description: Whether to enable the file manager

#### path
- Type: `string`
- Default: `"/__filemanager"`
- Description: File manager access path
- Note: Must start with `/`, cannot be root path `/`

#### password
- Type: `string`
- Default: none
- Description: Access password
- Requirement: At least 6 characters

#### allowOutsideRoot
- Type: `boolean`
- Default: `false`
- Description: Whether to allow access outside website root directory
- Security recommendation: Keep `false` unless you really need to access other directories

#### allowedPaths
- Type: `string[]`
- Default: `[]`
- Description: Allowed path whitelist
- Usage: When `allowOutsideRoot` is `true`, limit access to only these paths

#### deniedPaths
- Type: `string[]`
- Default: `[".git", ".deno", "node_modules", ".cache"]`
- Description: Denied path blacklist
- Usage: Any path containing matched patterns will be rejected

#### maxUploadSize
- Type: `number`
- Default: `104857600` (100MB)
- Description: Maximum upload file size in bytes

#### allowedExtensions
- Type: `string[]`
- Default: `[]`
- Description: Allowed upload file extension whitelist
- Usage: Empty list means allow all types (unless rejected by blacklist)

#### deniedExtensions
- Type: `string[]`
- Default: `[".exe", ".sh", ".bat", ".cmd", ".scr", ".pif"]`
- Description: Denied upload file extension blacklist

#### allowDelete
- Type: `boolean`
- Default: `true`
- Description: Whether to allow deleting files and directories

#### allowRename
- Type: `boolean`
- Default: `true`
- Description: Whether to allow renaming files and directories

#### allowMkdir
- Type: `boolean`
- Default: `true`
- Description: Whether to allow creating new directories

#### allowMove
- Type: `boolean`
- Default: `false`
- Description: Whether to allow moving files and directories

#### allowExtract
- Type: `boolean`
- Default: `true`
- Description: Whether to allow extracting compressed files

#### allowCompress
- Type: `boolean`
- Default: `true`
- Description: Whether to allow compressing files and directories

#### allowedArchiveExtensions
- Type: `ArchiveType[]`
- Default: `["tar", "tgz"]`
- Description: Allowed extract archive formats (zip is disabled by default due to binary compatibility issues)
- Optional values: `"tar"`, `"tgz"`

#### maxExtractSize
- Type: `number`
- Default: `1073741824` (1GB)
- Description: Maximum extract file size limit in bytes

#### maxCompressSize
- Type: `number`
- Default: `524288000` (500MB)
- Description: Maximum compress total file size limit in bytes

#### maxExtractFileCount
- Type: `number`
- Default: `10000`
- Description: ZIP bomb protection: maximum number of files allowed during extraction

## User Interface

### Login Page

On first access, the login page is displayed. Enter the configured password to log in.

### Main Interface

After logging in, you enter the file manager main interface:

- **Breadcrumb navigation**: Shows current path, clickable to navigate to parent directory
- **Upload button**: Click to open upload dialog, supports click-to-select or drag-and-drop upload
- **New directory button**: Create new directory
- **Logout button**: Log out

### File List

The file list displays the following information:

- **Icon**: File type icon
- **Name**: File name, directories are clickable to enter
- **Size**: File size
- **Modified time**: Last modified time
- **Actions**:
  - Download
  - Extract (compressed files only)
  - Rename (files only)
  - Delete (files only)
- **Checkbox**: Check files for batch operations

## API Endpoints

The file manager provides the following API endpoints (all APIs require authentication):

### POST `/__filemanager/api/login`
Login

**Request body**:
```json
{
  "password": "your-password"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "csrfToken": "csrf-token"
  }
}
```

### POST `/__filemanager/api/logout`
Logout

**Response**:
```json
{
  "success": true
}
```

### GET `/__filemanager/api/browse?path=/path/to/dir`
Browse directory

**Response**:
```json
{
  "success": true,
  "data": {
    "path": "/path/to/dir",
    "parentPath": "/path/to",
    "files": [
      {
        "name": "example.txt",
        "isDirectory": false,
        "size": 1024,
        "modifiedTime": "2024-01-15T10:30:00.000Z",
        "extension": ".txt"
      }
    ]
  }
}
```

### POST `/__filemanager/api/upload?path=/path/to/dir`
Upload file

**Request**: `multipart/form-data`
- `file`: The file

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "File uploaded successfully"
  }
}
```

### GET `/__filemanager/api/download?path=/path/to/file`
Download file

**Response**: File content

### POST `/__filemanager/api/delete`
Delete file or directory

**Request body**:
```json
{
  "path": "/path/to/file"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "Deleted successfully"
  }
}
```

### POST `/__filemanager/api/rename`
Rename file or directory

**Request body**:
```json
{
  "oldPath": "/path/to/oldname",
  "newName": "newname"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "Renamed successfully"
  }
}
```

### POST `/__filemanager/api/mkdir`
Create directory

**Request body**:
```json
{
  "parentPath": "/path/to/parent",
  "dirName": "newdir"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "Directory created successfully"
  }
}
```

### POST `/__filemanager/api/extract`
Extract compressed file

**Request body**:
```json
{
  "archivePath": "/path/to/archive.zip",
  "targetDir": "/path/to/destination"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "Extracted successfully"
  }
}
```

**Supported formats**: ZIP, TAR, TAR.GZ

### POST `/__filemanager/api/compress`
Compress files and directories to ZIP

**Request body**:
```json
{
  "sourcePaths": ["/path/to/file1", "/path/to/file2"],
  "targetPath": "/path/to/archive.zip",
  "includeSrc": false
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "Compressed successfully"
  }
}
```

### POST `/__filemanager/api/batch-move`
Batch move files and directories

**Request body**:
```json
{
  "sourcePaths": ["/path/to/file1", "/path/to/file2"],
  "targetDir": "/path/to/destination"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "Move completed: 2 succeeded, 0 failed",
    "successCount": 2,
    "failedCount": 0,
    "details": {
      "success": ["/path/to/file1", "/path/to/file2"],
      "failed": []
    }
  }
}
```

## Security Recommendations

### 1. Use Strong Passwords

Password should be at least 6 characters, recommend using longer and more complex passwords:

```jsonc
{
  "fileManager": {
    "password": "MyV3ryS3cur3P@ssw0rd!2024"
  }
}
```

### 2. Restrict Path Access

By default, the file manager can only access files within the website root directory. Do not enable `allowOutsideRoot` unless truly needed.

```jsonc
{
  "fileManager": {
    "allowOutsideRoot": false
  }
}
```

### 3. Configure Path Blacklist

Ensure sensitive directories are in the blacklist:

```jsonc
{
  "fileManager": {
    "deniedPaths": [
      ".git",
      ".deno",
      "node_modules",
      ".cache",
      ".env",
      "config"
    ]
  }
}
```

### 4. Restrict File Types

If only uploading specific file types is needed, configure a whitelist:

```jsonc
{
  "fileManager": {
    "allowedExtensions": [".jpg", ".png", ".gif", ".pdf", ".doc", ".docx"]
  }
}
```

### 5. Restrict Operation Permissions

If certain operations are not needed, disable them:

```jsonc
{
  "fileManager": {
    "allowDelete": false,
    "allowMove": false
  }
}
```

### 6. Use HTTPS

Production environment strongly recommends using HTTPS to protect passwords and sessions:

```bash
# Use Nginx or Caddy reverse proxy with HTTPS enabled
```

### 7. Regular Backups

Delete operations are irreversible, recommend regular backups:

```bash
# Regular backup script
tar -czf backup-$(date +%Y%m%d).tar.gz ./www
```

## Troubleshooting

### Problem: Cannot login after changing password

Modifying password in config file takes effect automatically without server restart.

**Solution**:
1. Modify password in config file
2. Refresh browser
3. Log in with new password
4. No server restart needed!

If still cannot login:
- Confirm password is at least 6 characters
- Check config file format is correct
- Check server logs to confirm config was loaded

### Problem: Cannot access file manager

**Possible causes**:
- File manager not enabled (`enabled: false`)
- Path configuration incorrect
- Password not set

**Solution**:
Check config file, ensure `fileManager.enabled` is `true` and password is set.

### Problem: Config file changes not taking effect

Config file supports auto-reload, no restart needed after changes.

**Verification method**:
1. Modify config file
2. Check server logs, should see "Config file modified, reloading"
3. Refresh browser, new config takes effect immediately

If not taking effect, check:
- Config file format is correct
- Using `.jsonc` extension if contains comments

### Problem: Shows "Not authenticated" after login

**Possible causes**:
- Cookies blocked
- Session expired

**Solution**:
Ensure browser allows cookies, re-login.

### Problem: Cannot upload files

**Possible causes**:
- File size exceeds limit
- File type is prohibited
- Insufficient target directory permissions

**Solution**:
Check `maxUploadSize`, `allowedExtensions`, `deniedExtensions` configurations.

### Problem: Cannot access certain directories

**Possible causes**:
- Directory in blacklist
- Outside root directory scope
- Path whitelist restrictions

**Solution**:
Check `deniedPaths`, `allowedPaths`, `allowOutsideRoot` configurations.

## Common Use Cases

### Use Case 1: Image Upload Only

```jsonc
{
  "fileManager": {
    "enabled": true,
    "password": "image-upload",
    "allowedExtensions": [".jpg", ".jpeg", ".png", ".gif", ".webp"],
    "allowDelete": false,
    "allowRename": false
  }
}
```

### Use Case 2: Multi-directory Management

```jsonc
{
  "fileManager": {
    "enabled": true,
    "password": "multi-dirs",
    "allowOutsideRoot": true,
    "allowedPaths": ["./www", "./uploads", "./backups"],
    "deniedPaths": [".git", ".env"]
  }
}
```

### Use Case 3: Read-only Mode (browse and download only)

```jsonc
{
  "fileManager": {
    "enabled": true,
    "password": "readonly",
    "allowDelete": false,
    "allowRename": false,
    "allowMkdir": false,
    "allowMove": false
  }
}
```

## Notes

1. **Path Traversal Protection**: File manager automatically blocks path traversal attacks (e.g., `../../../etc/passwd`)
2. **Hidden Files**: Hidden files starting with `.` are not shown by default
3. **Session Expiry**: Session validity is 2 hours, re-login required after expiry
4. **Concurrency Limits**: Current version has no concurrent upload limits, large file uploads may affect performance
5. **Logging**: All operations are recorded in TSP's access log

## Future Enhancements

## Extract/Compress Feature

### Supported Formats

- **ZIP**: Most common compression format
- **TAR**: Common archive format on Linux
- **TAR.GZ / TGZ**: GZIP compressed TAR format

### Extract Operation

1. Find compressed file (ZIP/TAR/TAR.GZ) in file list
2. Click the extract button after the file name
3. Confirm target directory in popup modal (default is current directory)
4. Click "Extract" button
5. File list auto-refreshes after extraction completes

### Compress Operation

1. Check checkboxes for one or more files/directories
2. Top toolbar shows "N items selected"
3. Click "Compress Selected" button
4. Enter ZIP filename in popup modal (e.g., archive.zip)
5. Optional: Check "Include parent directory" (preserves directory structure)
6. Click "Compress" button
7. File list auto-refreshes after compression completes

### Security Limits

- **File size limit**: Max 1GB for extraction, max 500MB total source file size for compression
- **File count limit**: Max 10,000 files per extraction (prevents ZIP bomb attacks)
- **Format limit**: Only allowed formats specified in config can be extracted
- **Path validation**: All paths are security-checked to prevent path traversal attacks

## Batch Operations

### Batch Selection

- In the file list, each file/directory has a checkbox in front
- After checking one or more files, the top toolbar shows "N items selected"
- Toolbar buttons automatically enable/disable based on selection count

### Batch Move

1. Check checkboxes for files/directories to move
2. Click "Move Selected" button in top toolbar
3. Enter target directory path in popup modal:
   - **Absolute path**: Complete path from root directory
   - **Relative path**: Path relative to current directory (e.g., `../subdir`)
4. Click "Move" button
5. System moves files one by one, showing move results (success/failure count)
6. File list auto-refreshes after move completes

**Notes**:
- Target directory must exist, system will not create automatically
- If file with same name already exists in target directory, it will be skipped and counted as failure
- Move operation is irreversible, please operate with caution
- Move is enabled by default (`allowMove: true`), to disable set `allowMove: false` in config

### Batch Delete

1. Check checkboxes for files/directories to delete
2. Click "Delete Selected" button in top toolbar
3. Confirm delete operation
4. System deletes files one by one, showing delete results (success/failure count)
5. File list auto-refreshes after delete completes

**Notes**:
- Delete operation is irreversible, please operate with caution
- Directory deletion recursively deletes all contents
- Requires `allowDelete: true` enabled in config to use this feature

### Batch Compress

See "Extract/Compress Feature" section's "Compress Operation" part.

## Planned Enhancements

Planned feature improvements:

- Chunked upload for large files
- File preview (images, text, PDF)
- Batch operations (select multiple files)
- Search functionality
- Thumbnail generation
- Path-based permission configuration
- Multi-user support
- Operation audit logging

## Implementation

The file manager uses the following technologies:

- **Backend**: Deno + TypeScript
- **Frontend**: Native HTML + CSS + JavaScript (no framework dependencies)
- **Authentication**: PBKDF2 password hashing + Session management
- **Security**: Path validation, CSRF Token, file type checking

## License

TSP File Manager follows the same license as TSP.
