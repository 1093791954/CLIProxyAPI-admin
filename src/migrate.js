import { db } from './db.js';
import { clampNonNegative, maskKey, newId, nowIso } from './utils.js';

const VALID_STATUS = new Set(['active', 'disabled', 'exhausted', 'deleted']);

function tableExists(name) {
  const row = db.raw
    .prepare(`SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?`)
    .get(name);
  return Boolean(row);
}

function tableSql(name) {
  const row = db.raw
    .prepare(`SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?`)
    .get(name);
  return row?.sql || '';
}

function columnSet(tableName) {
  const rows = db.raw.prepare(`PRAGMA table_info(${tableName})`).all();
  return new Set(rows.map((item) => item.name));
}

function createApiKeysTable(tableName = 'api_keys') {
  db.raw.exec(`
    CREATE TABLE ${tableName} (
      id TEXT PRIMARY KEY,
      key_plaintext TEXT NOT NULL UNIQUE,
      key_mask TEXT NOT NULL,
      remark TEXT NOT NULL DEFAULT '',
      status TEXT NOT NULL CHECK (status IN ('active', 'disabled', 'exhausted', 'deleted')),
      total_quota_tokens INTEGER NOT NULL DEFAULT 0,
      used_tokens INTEGER NOT NULL DEFAULT 0,
      remaining_tokens INTEGER NOT NULL DEFAULT 0,
      rpm_limit INTEGER NOT NULL DEFAULT 0,
      tpm_limit INTEGER NOT NULL DEFAULT 0,
      expires_at TEXT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      disabled_at TEXT NULL,
      exhausted_at TEXT NULL,
      deleted_at TEXT NULL,
      remote_exists INTEGER NOT NULL DEFAULT 1,
      sync_status TEXT NOT NULL DEFAULT 'pending'
    )
  `);
}

function normalizeStatus(raw) {
  const value = String(raw || '').trim().toLowerCase();
  if (VALID_STATUS.has(value)) return value;
  if (value === 'inactive' || value === 'blocked') return 'disabled';
  return 'active';
}

function normalizeApiKeyRow(row) {
  const now = nowIso();
  const keyPlaintext = String(row?.key_plaintext || row?.key || '').trim();
  if (!keyPlaintext) return null;

  const status = normalizeStatus(row?.status);
  const total = clampNonNegative(row?.total_quota_tokens);
  const used = clampNonNegative(row?.used_tokens);
  const remainingRaw = row?.remaining_tokens;
  const remaining =
    remainingRaw === null || remainingRaw === undefined
      ? total > 0
        ? Math.max(0, total - used)
        : 0
      : clampNonNegative(remainingRaw);

  const remoteExists =
    status === 'active'
      ? Number(row?.remote_exists ?? 1) ? 1 : 0
      : 0;

  let syncStatus = String(row?.sync_status || '').trim();
  if (!syncStatus) {
    if (status === 'active') syncStatus = remoteExists ? 'ok' : 'missing_remote';
    if (status === 'disabled') syncStatus = 'disabled';
    if (status === 'exhausted') syncStatus = 'exhausted';
    if (status === 'deleted') syncStatus = 'deleted';
  }

  return {
    id: row?.id || newId('key_'),
    keyPlaintext,
    keyMask: row?.key_mask || maskKey(keyPlaintext),
    remark: String(row?.remark || ''),
    status,
    total,
    used,
    remaining,
    rpm: clampNonNegative(row?.rpm_limit),
    tpm: clampNonNegative(row?.tpm_limit),
    expiresAt: row?.expires_at || null,
    createdAt: row?.created_at || now,
    updatedAt: row?.updated_at || now,
    disabledAt: row?.disabled_at || null,
    exhaustedAt: row?.exhausted_at || null,
    deletedAt: row?.deleted_at || null,
    remoteExists,
    syncStatus
  };
}

function rebuildApiKeysTable() {
  const rows = db.raw.prepare('SELECT * FROM api_keys').all();
  const normalized = rows.map(normalizeApiKeyRow).filter(Boolean);

  db.raw.exec('BEGIN');
  try {
    createApiKeysTable('api_keys_new');

    const insert = db.raw.prepare(`
      INSERT INTO api_keys_new (
        id, key_plaintext, key_mask, remark, status,
        total_quota_tokens, used_tokens, remaining_tokens,
        rpm_limit, tpm_limit, expires_at,
        created_at, updated_at, disabled_at, exhausted_at, deleted_at,
        remote_exists, sync_status
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    for (const item of normalized) {
      insert.run(
        item.id,
        item.keyPlaintext,
        item.keyMask,
        item.remark,
        item.status,
        item.total,
        item.used,
        item.remaining,
        item.rpm,
        item.tpm,
        item.expiresAt,
        item.createdAt,
        item.updatedAt,
        item.disabledAt,
        item.exhaustedAt,
        item.deletedAt,
        item.remoteExists,
        item.syncStatus
      );
    }

    db.raw.exec('DROP TABLE api_keys');
    db.raw.exec('ALTER TABLE api_keys_new RENAME TO api_keys');
    db.raw.exec('COMMIT');
  } catch (error) {
    db.raw.exec('ROLLBACK');
    throw error;
  }
}

function ensureApiKeysSchema() {
  if (!tableExists('api_keys')) {
    createApiKeysTable('api_keys');
    return;
  }

  const cols = columnSet('api_keys');
  const sql = tableSql('api_keys');
  const hasExtendedStatus = sql.includes('exhausted') && sql.includes('deleted');
  const required = [
    'id',
    'key_plaintext',
    'key_mask',
    'remark',
    'status',
    'total_quota_tokens',
    'used_tokens',
    'remaining_tokens',
    'rpm_limit',
    'tpm_limit',
    'expires_at',
    'created_at',
    'updated_at',
    'disabled_at',
    'exhausted_at',
    'deleted_at',
    'remote_exists',
    'sync_status'
  ];

  const hasRequired = required.every((col) => cols.has(col));

  if (!hasRequired || !hasExtendedStatus) {
    rebuildApiKeysTable();
  }
}

export async function migrate() {
  ensureApiKeysSchema();

  db.raw.exec(`
    CREATE TABLE IF NOT EXISTS usage_sync_cursor (
      id INTEGER PRIMARY KEY CHECK (id = 1),
      last_sync_at TEXT NOT NULL,
      last_event_id TEXT NULL
    )
  `);

  db.raw.exec(`
    CREATE TABLE IF NOT EXISTS announcements (
      id TEXT PRIMARY KEY,
      title TEXT NOT NULL,
      content_md TEXT NOT NULL,
      is_current INTEGER NOT NULL DEFAULT 0,
      version INTEGER NOT NULL,
      created_at TEXT NOT NULL,
      created_by TEXT NOT NULL
    )
  `);

  db.raw.exec(`
    CREATE TABLE IF NOT EXISTS audit_logs (
      id TEXT PRIMARY KEY,
      actor TEXT NOT NULL,
      action TEXT NOT NULL,
      target_key_id TEXT NULL,
      payload_json TEXT NOT NULL,
      created_at TEXT NOT NULL
    )
  `);

  db.raw.exec(`CREATE INDEX IF NOT EXISTS idx_api_keys_status ON api_keys(status)`);
  db.raw.exec(`CREATE INDEX IF NOT EXISTS idx_api_keys_created_at ON api_keys(created_at)`);
  db.raw.exec(`CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at)`);

  db.raw
    .prepare(`INSERT OR IGNORE INTO usage_sync_cursor (id, last_sync_at, last_event_id) VALUES (1, ?, NULL)`)
    .run(new Date(0).toISOString());
}
