#!/usr/bin/env bash
set -euo pipefail

APP_DIR=/opt/liteadmin

mkdir -p "$APP_DIR"
cd "$APP_DIR"

if ! command -v node >/dev/null 2>&1; then
  echo "Node.js 未安装，请先安装 Node.js 20+"
  exit 1
fi

if [ ! -f package.json ]; then
  echo "请先把项目文件上传到 $APP_DIR"
  exit 1
fi

npm install --omit=dev
cp -n .env.example .env
npm run db:migrate
npm run db:seed-admin

cp deploy/liteadmin.service /etc/systemd/system/liteadmin.service
systemctl daemon-reload
systemctl enable --now liteadmin.service

cp deploy/nginx-8319.conf /etc/nginx/sites-available/liteadmin-8319
ln -sf /etc/nginx/sites-available/liteadmin-8319 /etc/nginx/sites-enabled/liteadmin-8319
nginx -t
systemctl reload nginx

echo "部署完成：请访问 http://<你的服务器IP>:8319/check"
