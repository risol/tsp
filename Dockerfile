# 使用官方 Deno 镜像作为构建环境（使用最新版本以支持 lockfile v5）
FROM denoland/deno:latest

# 添加元数据
LABEL maintainer="TSP Server"
LABEL description="TSP Server - Linux x64 Binary Builder"
LABEL version="1.0.0"

# 设置工作目录
WORKDIR /app

# 复制项目文件
COPY . .

# 编译 Linux x64 二进制文件
RUN deno compile --allow-all --output tspserver --target x86_64-unknown-linux-gnu src/main.ts

# 创建发布目录结构
RUN mkdir -p dist/tspserver && \
    cp tspserver dist/tspserver/ && \
    cp -r www dist/tspserver/ && \
    cp README.md dist/tspserver/ 2>/dev/null || true && \
    cp deno.json dist/tspserver/ 2>/dev/null || true

# 设置权限
RUN chmod +x dist/tspserver/tspserver

# 创建打包信息文件
RUN echo "Build Date: $(date -u +'%Y-%m-%d %H:%M:%S UTC')" > dist/tspserver/BUILD_INFO.txt && \
    echo "Deno Version: $(deno --version | head -1)" >> dist/tspserver/BUILD_INFO.txt && \
    echo "Target: x86_64-unknown-linux-gnu" >> dist/tspserver/BUILD_INFO.txt && \
    echo "Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')" >> dist/tspserver/BUILD_INFO.txt

# 打包为 tar.gz
RUN cd dist && tar -czf ../tspserver-linux-x64.tar.gz tspserver

# 创建输出目录
RUN mkdir -p /output

# 复制构建产物到输出目录
RUN cp tspserver-linux-x64.tar.gz /output/

# 显示构建信息
RUN echo "Build completed successfully!" && \
    ls -lh /output/tspserver-linux-x64.tar.gz

# 默认命令显示构建结果
CMD ["ls", "-lh", "/output/"]
