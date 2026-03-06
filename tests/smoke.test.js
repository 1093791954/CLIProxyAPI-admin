import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import assert from 'node:assert/strict';

const testDb = path.join(process.cwd(), 'data', 'test-cliproxy-admin.db');
process.env.DB_PATH = testDb;
process.env.JWT_SECRET = 'test-secret';
process.env.ADMIN_ACCESS_KEY = 'test-access-key';
process.env.CLIPROXY_BASE_URL = 'http://127.0.0.1:8317';
process.env.CLIPROXY_MANAGEMENT_KEY = '123';

let server;
let baseUrl;

test.before(async () => {
  if (fs.existsSync(testDb)) fs.unlinkSync(testDb);
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

test('admin login success', async () => {
  const res = await fetch(`${baseUrl}/api/admin/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ accessKey: 'test-access-key' })
  });
  assert.equal(res.status, 200);
  const data = await res.json();
  assert.ok(data.accessToken);
});
