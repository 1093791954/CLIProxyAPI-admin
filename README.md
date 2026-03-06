# CLIProxy Key Admin

Independent key management site for CLIProxyAPI.

- Single access-key login for admin (no username/password)
- Key create / update / disable / delete / query
- Local quota management with remote usage sync
- Auto-disable by removing key from CLIProxy when quota exhausted
- Announcement management
- Audit logs
- Public key check page

## Requirements

- Node.js 20+
- SQLite
- Reachable CLIProxyAPI management endpoint

## Start

```bash
cp .env.example .env
npm ci
npm run db:migrate
npm start
```

Pages:

- Public check page: `/check`
- Admin login page: `/admin/login`

## Important Env

- `ADMIN_ACCESS_KEY`: admin login secret
- `CLIPROXY_BASE_URL`: e.g. `http://127.0.0.1:8317`
- `CLIPROXY_MANAGEMENT_KEY`: CLIProxy management key
- `PUBLIC_RATE_LIMIT_MAX`: public check limit per minute
- `SYNC_INTERVAL_MS`: usage sync interval (ms)
