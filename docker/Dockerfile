# 多阶段构建 TSP Server Docker 镜像
# 第一阶段：构建 Linux 二进制文件

FROM denoland/deno:latest AS builder

# 设置工作目录
WORKDIR /app

# 复制项目文件
COPY . .

# 编译 Linux x64 二进制文件
RUN deno compile --allow-all --output /tmp/tspserver --target x86_64-unknown-linux-gnu src/main.ts

# 第二阶段：创建轻量级运行镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# 创建非特权用户
RUN groupadd -r tspserver && \
    useradd -r -g tspserver tspserver

# 创建应用目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /tmp/tspserver /usr/local/bin/tspserver

# 复制网站文件
COPY www/ ./www/

# 设置正确的权限
RUN chown -R tspserver:tspserver /app && \
    chmod +x /usr/local/bin/tspserver && \
    chmod -R 755 /app/www

# 切换到非特权用户
USER tspserver

# 暴露默认端口
EXPOSE 9000

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9000/ || exit 1

# 设置默认环境变量
ENV TSP_PORT=9000 \
    TSP_ROOT=/app/www \
    TSP_MODE=Production

# 启动服务器
CMD ["tspserver", "--root", "/app/www", "--port", "9000"]
