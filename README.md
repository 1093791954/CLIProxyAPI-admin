# CLIProxyAPI-admin

Rust 实现的本地管理服务，提供：

- API Key 创建 / 列表 / 编辑 / 停用 / 启用 / 删除
- Key Token 总量限额（累计）与自动停用
- OpenAI v1 前置网关（转发到 `CLIProxyAPI`）
- 管理页面（SSR）
- SQLite 本地轻量数据库

## 运行

1. 复制环境变量模板

```bash
copy .env.example .env
```

2. 设置 `.env`（至少要改 `UPSTREAM_BEARER_KEY`）

```env
BIND_ADDR=127.0.0.1:8318
DATABASE_URL=sqlite://./data/admin.db
UPSTREAM_BASE_URL=http://localhost:8317
UPSTREAM_BEARER_KEY=replace-with-your-upstream-key
RUST_LOG=info
```

3. 启动

```bash
cargo run
```

4. 访问

- 管理页：`http://127.0.0.1:8318/admin`
- 健康检查：`http://127.0.0.1:8318/health`

## 对外接口

### 管理 API

- `POST /admin/api/keys`
- `GET /admin/api/keys`
- `PATCH /admin/api/keys/{id}`
- `DELETE /admin/api/keys/{id}`
- `POST /admin/api/keys/{id}/disable`
- `POST /admin/api/keys/{id}/enable`
- `POST /admin/api/keys/{id}/reset-usage`
- `GET /admin/api/keys/{id}/usage-events`

### 前置网关

- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/completions`
- `POST /v1/responses`

说明：
- 客户端请求 `Authorization: Bearer <admin_key>`
- 服务端转发到 `UPSTREAM_BASE_URL`，并替换为固定上游 `UPSTREAM_BEARER_KEY`
- `GET /v1/responses`（WebSocket）当前返回 `501`

## 使用 jshook-reverse 调试管理页面

先手动启动 Chrome 调试端口：

```bash
"C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222
```

然后在技能目录执行：

```bash
node dist/skill.js browser launch
node dist/skill.js page goto http://127.0.0.1:8318/admin
node dist/skill.js debugger enable
node dist/skill.js hook generate fetch */admin/api/*
node dist/skill.js hook-data
```
