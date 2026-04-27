# Production Binary Deployment

## 1. 目标模型

MVP 生产部署主路径是：

- 单个 Linux server binary
- 前端静态构建产物 `web/`
- 数据库迁移目录 `migrations/`
- 外部 PostgreSQL 16+
- 独立入口域名，例如 `config-center.mycompany.com`
- 由现有反向代理 / TLS 基础设施负责 HTTPS、证书和域名转发

不把 PostgreSQL、DNS、证书或反向代理纳入项目编排。`docker-compose.yml` 不作为 MVP 生产交付目标。

## 2. Release Artifact Layout

建议发布包结构：

```text
mini-conf/
  bin/
    mini-conf-server
  web/
    index.html
    assets/
  migrations/
    0001_*.sql
    ...
  config/
    mini-conf.env.example
  systemd/
    mini-conf.service.example
  RELEASE.txt
```

当前 server crate 的 release binary 名称是 `server`。发布时建议重命名为 `mini-conf-server`，避免生产机器上出现语义不清的进程名。

## 3. Build Artifact

在 CI 或发布机上构建：

```bash
pnpm install --frozen-lockfile
just release-package
```

默认输出：

```text
dist/mini-conf-linux-x86_64.tar.gz
```

可用 `MINI_CONF_RELEASE_NAME` 覆盖包名，用 `MINI_CONF_DIST_DIR` 覆盖输出目录。

GitHub Actions 也提供 `Release Package` workflow，可手动触发，或在推送 `v*` tag 时生成并上传同名 artifact。

发布包不包含 PostgreSQL 数据目录，不包含生产密钥，不包含 TLS 证书。

## 4. External PostgreSQL

部署方需要提前准备：

- PostgreSQL 16+
- 独立 database，例如 `mini_conf`
- 独立 database user，例如 `mini_conf_app`
- 可从应用主机访问的连接串
- 备份、恢复、监控和容量策略

应用只通过 `DATABASE_URL` 连接外部 PostgreSQL，不负责创建或管理 PostgreSQL 实例。

迁移独立执行，不随生产服务进程自动运行：

```bash
export DATABASE_URL='postgres://mini_conf_app:***@postgres.example:5432/mini_conf'
sqlx migrate run --source /opt/mini-conf/migrations
```

`APP_ENV=prod` 时服务会在启动时连接 `DATABASE_URL`，但不会自动运行 migrations 或 seed。

## 5. Environment File

发布包内提供 `config/mini-conf.env.example`，生产部署时写入 `/etc/mini-conf/mini-conf.env`：

```bash
APP_ENV=prod
HTTP_ADDR=127.0.0.1:8080
DATABASE_URL=postgres://mini_conf_app:***@postgres.example:5432/mini_conf
STATIC_DIR=/opt/mini-conf/web
INIT_DB_ON_BOOT=false
RUST_LOG=info
```

说明：

- `APP_ENV=prod` 默认启用 secure cookie 和 HSTS。
- `INIT_DB_ON_BOOT=false` 是生产要求；生产迁移和初始化必须独立执行。
- `DATABASE_URL` 必须显式提供。
- `HTTP_ADDR` 建议只监听本机或内网地址，由反向代理暴露公网 HTTPS。

## 6. systemd Unit

发布包内提供 `systemd/mini-conf.service.example`。生产部署时可复制为 `/etc/systemd/system/mini-conf.service`：

```ini
[Unit]
Description=mini-conf config center
After=network-online.target
Wants=network-online.target

[Service]
User=mini-conf
Group=mini-conf
WorkingDirectory=/opt/mini-conf
EnvironmentFile=/etc/mini-conf/mini-conf.env
ExecStart=/opt/mini-conf/bin/mini-conf-server
Restart=on-failure
RestartSec=5
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

启动：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now mini-conf
sudo systemctl status mini-conf
```

## 7. Reverse Proxy

入口域名建议：

```text
config-center.mycompany.com
```

Nginx 示例：

```nginx
server {
    listen 443 ssl http2;
    server_name config-center.mycompany.com;

    ssl_certificate     /etc/nginx/certs/config-center.mycompany.com.crt;
    ssl_certificate_key /etc/nginx/certs/config-center.mycompany.com.key;

    client_max_body_size 2m;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

如果公司已有通配符证书或统一网关，应优先复用现有 TLS 和路由能力。

## 8. First Boot

首次部署顺序：

1. 准备外部 PostgreSQL database 和账号。
2. 上传并解压 release artifact 到 `/opt/mini-conf`。
3. 写入 `/etc/mini-conf/mini-conf.env`。
4. 运行 migrations。
5. 启动 `mini-conf.service`。
6. 访问 `https://config-center.mycompany.com/api/healthz`。
7. 访问管理台完成 setup。

## 9. Smoke Checks

```bash
curl -fsS https://config-center.mycompany.com/api/healthz
curl -fsSI https://config-center.mycompany.com/
```

完成 setup 后再验证：

- 管理端登录
- 项目创建
- 部署实例 token 签发
- Open API `GET /api/open/configs/resolve`

## 10. Rollback

推荐保留上一个 release 目录：

```text
/opt/mini-conf/releases/20260425-120000/
/opt/mini-conf/current -> /opt/mini-conf/releases/20260425-120000/
```

回滚应用时切换 `current` symlink 并重启 systemd。数据库迁移回滚必须单独评估，不默认随应用包自动回滚。

## 11. Optional Docker Image

Docker image 可以作为后续可选包装形式，用于接入容器平台或统一镜像发布流程。

即使使用 Docker image，生产模型仍保持：

- PostgreSQL 是外部基础设施
- 不提供生产 `docker-compose.yml`
- migrations 独立执行
- DNS / TLS / 反向代理由部署环境管理
