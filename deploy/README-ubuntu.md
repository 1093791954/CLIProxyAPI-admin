# Ubuntu 部署步骤（8319）

以下命令在服务器执行。

## 1. 上传项目

将当前目录全部上传到：`/opt/liteadmin`

## 2. 安装依赖并初始化

```bash
cd /opt/liteadmin
cp -n .env.example .env
# 编辑 .env：至少填写 LITELLM_BASE_URL / LITELLM_MASTER_KEY / JWT_SECRET
npm install --omit=dev
npm run db:migrate
npm run db:seed-admin
```

## 3. 配置 systemd

```bash
cp deploy/liteadmin.service /etc/systemd/system/liteadmin.service
systemctl daemon-reload
systemctl enable --now liteadmin.service
systemctl status liteadmin.service --no-pager
```

## 4. 配置 Nginx 监听 8319

```bash
cp deploy/nginx-8319.conf /etc/nginx/sites-available/liteadmin-8319
ln -sf /etc/nginx/sites-available/liteadmin-8319 /etc/nginx/sites-enabled/liteadmin-8319
nginx -t
systemctl reload nginx
```

## 5. 验证

```bash
curl http://127.0.0.1:3001/api/health
curl http://127.0.0.1:8319/api/health
```

## 6. 常用维护

```bash
journalctl -u liteadmin.service -f
npm run import:keys
```
