# 快速构建 Linux 二进制文件

## Windows

双击运行 `build-linux-binary.bat` 即可自动完成构建。

## Linux / macOS

```bash
chmod +x build-linux-binary.sh
./build-linux-binary.sh
```

## 构建产物

成功后会在 `output/` 目录生成 `tspserver-linux-x64.tar.gz`（约 38MB）。

## 部署到 Linux 服务器

```bash
# 1. 传输文件（在本地执行）
scp output/tspserver-linux-x64.tar.gz user@your-server:/opt/

# 2. 解压（在 Linux 服务器上执行）
cd /opt
tar -xzf tspserver-linux-x64.tar.gz
cd tspserver

# 3. 运行
./tspserver --root ./www --port 9000
```

## 详细文档

请查看 [BUILD_LINUX.md](BUILD_LINUX.md) 获取更多详细信息。
