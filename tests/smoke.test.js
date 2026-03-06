import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import test from 'node:test';
import assert from 'node:assert/strict';

const testDb = path.join(process.cwd(), 'data', 'test-cliproxy-admin.db');
process.env.DB_PATH = testDb;
process.env.JWT_SECRET = 'test-secret';
process.env.ADMIN_ACCESS_KEY = 'test-access-key';
process.env.CLIPROXY_MANAGEMENT_KEY = '123';

let server;
let baseUrl;
let managementServer;
let remoteKeys = [];
let usageByKey = new Map();

function json(res, status, payload) {
  res.statusCode = status;
  res.setHeader('Content-Type', 'application/json; charset=utf-8');
  res.end(JSON.stringify(payload));
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = '';
    req.setEncoding('utf8');
    req.on('data', (chunk) => {
      data += chunk;
    });
    req.on('end', () => resolve(data));
    req.on('error', reject);
  });
}

async function startManagementServer() {
  managementServer = http.createServer(async (req, res) => {
    const url = new URL(req.url, 'http://127.0.0.1');

    if (req.method === 'GET' && url.pathname === '/v0/management/api-keys') {
      return json(res, 200, { 'api-keys': remoteKeys });
    }

    if (req.method === 'PUT' && url.pathname === '/v0/management/api-keys') {
      const body = await readBody(req);
      const payload = body ? JSON.parse(body) : [];
      remoteKeys = [...new Set((payload || []).map((item) => String(item || '').trim()).filter(Boolean))];
      return json(res, 200, { 'api-keys': remoteKeys });
    }

    if (req.method === 'GET' && url.pathname === '/v0/management/usage') {
      const usage = Object.fromEntries(
        remoteKeys.map((key) => [key, { total_tokens: usageByKey.get(key) || 0 }])
      );
      return json(res, 200, { usage: { apis: usage } });
    }

    return json(res, 404, { error: 'not found' });
  });

  await new Promise((resolve) => {
    managementServer.listen(0, '127.0.0.1', resolve);
  });

  const address = managementServer.address();
  process.env.CLIPROXY_BASE_URL = `http://127.0.0.1:${address.port}`;
}

async function loginAdmin() {
  const res = await fetch(`${baseUrl}/api/admin/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ accessKey: 'test-access-key' })
  });
  assert.equal(res.status, 200);
  const data = await res.json();
  assert.ok(data.accessToken);
  return data.accessToken;
}

async function adminFetch(pathname, options = {}) {
  const token = await loginAdmin();
  return fetch(`${baseUrl}${pathname}`, {
    ...options,
    headers: {
      Authorization: `Bearer ${token}`,
      ...(options.headers || {})
    }
  });
}

async function resetKeys() {
  const { db } = await import('../src/db.js');
  await db.run('DELETE FROM api_keys');
  await db.run('DELETE FROM audit_logs');
  remoteKeys = [];
  usageByKey = new Map();
}

async function insertKeyRow({
  id,
  keyPlaintext,
  remark,
  status,
  totalQuotaTokens,
  usedTokens,
  remainingTokens,
  remoteExists,
  syncStatus,
  deletedAt = null,
  exhaustedAt = null,
  disabledAt = null
}) {
  const { db } = await import('../src/db.js');
  const now = new Date().toISOString();
  await db.run(
    `INSERT INTO api_keys (
      id, key_plaintext, key_mask, remark, status,
      total_quota_tokens, used_tokens, remaining_tokens,
      rpm_limit, tpm_limit, expires_at,
      created_at, updated_at, disabled_at, exhausted_at, deleted_at,
      remote_exists, sync_status
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, 0, NULL, ?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      keyPlaintext,
      '****',
      remark,
      status,
      totalQuotaTokens,
      usedTokens,
      remainingTokens,
      now,
      now,
      disabledAt,
      exhaustedAt,
      deletedAt,
      remoteExists ? 1 : 0,
      syncStatus
    ]
  );
}

test.before(async () => {
  if (fs.existsSync(testDb)) fs.unlinkSync(testDb);
  await startManagementServer();
  const { createApp } = await import('../src/app.js');
  const app = await createApp();
  await new Promise((resolve) => {
    server = app.listen(0, '127.0.0.1', resolve);
  });
  const addr = server.address();
  baseUrl = `http://127.0.0.1:${addr.port}`;
});

test.after(async () => {
  await new Promise((resolve, reject) => {
    server.close((err) => (err ? reject(err) : resolve()));
  });
  await new Promise((resolve, reject) => {
    managementServer.close((err) => (err ? reject(err) : resolve()));
  });
  const { db } = await import('../src/db.js');
  db.raw.close();
  if (fs.existsSync(testDb)) fs.unlinkSync(testDb);
});

test('health endpoint', async () => {
  const res = await fetch(`${baseUrl}/api/health`);
  assert.equal(res.status, 200);
  const json = await res.json();
  assert.equal(json.ok, true);
});

test('root path returns no content', async () => {
  const res = await fetch(`${baseUrl}/`);
  assert.equal(res.status, 204);
  const text = await res.text();
  assert.equal(text, '');
});

test('admin entry serves login page', async () => {
  const res = await fetch(`${baseUrl}/admin`);
  assert.equal(res.status, 200);
  const html = await res.text();
  assert.match(html, /\/api\/admin\/auth\/login/);
  assert.match(html, /\/admin\/dashboard/);
});

test('admin dashboard serves management page', async () => {
  const res = await fetch(`${baseUrl}/admin/dashboard`);
  assert.equal(res.status, 200);
  const html = await res.text();
  assert.match(html, /\/api\/admin\/keys/);
});

test('check page remains available', async () => {
  const res = await fetch(`${baseUrl}/check`);
  assert.equal(res.status, 200);
  const html = await res.text();
  assert.match(html, /\/api\/public\/key\/check/);
});

test('admin login success', async () => {
  const token = await loginAdmin();
  assert.ok(token);
});

test('default key list hides deleted records and disables caching', async () => {
  await resetKeys();
  remoteKeys = ['sk-live'];

  await insertKeyRow({
    id: 'key_live',
    keyPlaintext: 'sk-live',
    remark: 'live',
    status: 'active',
    totalQuotaTokens: 100,
    usedTokens: 0,
    remainingTokens: 100,
    remoteExists: true,
    syncStatus: 'ok'
  });

  await insertKeyRow({
    id: 'key_deleted',
    keyPlaintext: 'sk-deleted',
    remark: 'legacy deleted',
    status: 'deleted',
    totalQuotaTokens: 100,
    usedTokens: 0,
    remainingTokens: 100,
    remoteExists: false,
    syncStatus: 'deleted',
    deletedAt: new Date().toISOString()
  });

  const res = await adminFetch('/api/admin/keys');
  assert.equal(res.status, 200);
  assert.match(res.headers.get('cache-control') || '', /no-store/);

  const data = await res.json();
  assert.equal(data.total, 1);
  assert.equal(data.items.length, 1);
  assert.equal(data.items[0].id, 'key_live');
  assert.equal(data.items.some((item) => item.id === 'key_deleted'), false);

  const deletedRes = await adminFetch('/api/admin/keys?status=deleted');
  assert.equal(deletedRes.status, 200);
  const deletedData = await deletedRes.json();
  assert.equal(deletedData.total, 0);
  assert.equal(deletedData.items.length, 0);
});

test('deleting a key removes it locally and remotely', async () => {
  await resetKeys();
  remoteKeys = ['sk-delete-me'];

  await insertKeyRow({
    id: 'key_delete_me',
    keyPlaintext: 'sk-delete-me',
    remark: 'to delete',
    status: 'active',
    totalQuotaTokens: 100,
    usedTokens: 0,
    remainingTokens: 100,
    remoteExists: true,
    syncStatus: 'ok'
  });

  const deleteRes = await adminFetch('/api/admin/keys/key_delete_me', {
    method: 'DELETE'
  });
  assert.equal(deleteRes.status, 200);
  const deleted = await deleteRes.json();
  assert.deepEqual(deleted, {
    ok: true,
    id: 'key_delete_me',
    keyPlaintext: 'sk-delete-me'
  });

  assert.deepEqual(remoteKeys, []);

  const { db } = await import('../src/db.js');
  const row = await db.get(`SELECT id FROM api_keys WHERE id = ?`, ['key_delete_me']);
  assert.equal(row, null);

  const listRes = await adminFetch('/api/admin/keys');
  assert.equal(listRes.status, 200);
  const listData = await listRes.json();
  assert.equal(listData.total, 0);
  assert.equal(listData.items.length, 0);
});
