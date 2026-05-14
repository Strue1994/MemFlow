# FIX-05: Docker Containers Running as Root

## 问题描述

Docker 容器默认以 root 用户运行，存在容器逃逸风险。

## 当前配置

`docker-compose.yml` 中没有指定用户:

```yaml
services:
  memflow:
    image: memflow/executor:latest
    # 没有 user 配置
```

## 修复方案

1. 在 docker-compose 中添加安全配置:
   ```yaml
   services:
     memflow:
       image: memflow/executor:latest
       user: "1000:1000"  # 非 root 用户
       security_opt:
         - no-new-privileges:true
       read_only: true    # 只读文件系统
       tmpfs:
         - /tmp:exec,size=100M
   ```

2. 创建非 root 用户 Dockerfile:
   ```dockerfile
   FROM rust:1.75 as builder
   # ... build steps
   
   FROM debian:bookworm-slim
   RUN groupadd -g 1000 memflow && useradd -u 1000 -g memflow memflow
   COPY --from=builder /app/target/release/memflow /usr/local/bin/
   USER memflow
   CMD ["memflow"]
   ```

3. 添加 capabilities 限制

## 影响文件

- `docker-compose.yml`
- `Dockerfile`

## 验证方法

确认容器内进程不以 root 运行: `docker exec <container> id`

## 优先级

MEDIUM - 安全加固