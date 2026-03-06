import { db } from '../db.js';
import { getApiKeys, getUsage, replaceApiKeys } from '../cliproxyClient.js';
import { clampNonNegative, generateApiKey, maskKey, newId, nowIso, toInt } from '../utils.js';
import { config } from '../config.js';

const VALID_STATUS = new Set(['active', 'disabled', 'exhausted', 'deleted']);

let mutationQueue = Promise.resolve();

function withMutation(task) {
  const next = mutationQueue.then(task, task);
  mutationQueue = next.catch(() => undefined);
  return next;
}

function removeKeyInArray(keys, key) {
  const idx = keys.indexOf(key);
  if (idx < 0) return false;
  keys.splice(idx, 1);
  return true;
}

function normalizeStatus(status) {
  const value = String(status || '').trim().toLowerCase();
  if (VALID_STATUS.has(value)) return value;
  return 'active';
}

function normalizeKeyInput(key) {
  return String(key || '').trim();
}

function normalizeDateInput(value) {
  const raw = String(value ?? '').trim();
  return raw || null;
}

function usageForKey(usagePayload, key) {
  const root = usagePayload?.usage ?? usagePayload ?? {};
  const node = root?.apis?.[key];
  const n = Number(node?.total_tokens ?? 0);
  return Number.isFinite(n) && n > 0 ? Math.floor(n) : 0;
}

function computeRemaining(total, used) {
  if (total <= 0) return 0;
  return Math.max(0, total - used);
}

function deriveSyncStatus(status, remoteExists) {
  if (status === 'active') return remoteExists ? 'ok' : 'missing_remote';
  if (status === 'disabled') return 'disabled';
  if (status === 'exhausted') return 'exhausted';
  if (status === 'deleted') return 'deleted';
  return 'pending';
}

function mapRow(row, remoteSet = null) {
  if (!row) return null;

  const status = normalizeStatus(row.status);
  const remoteExists =
    remoteSet instanceof Set
      ? remoteSet.has(row.key_plaintext)
      : Boolean(row.remote_exists);

  return {
    id: row.id,
    keyPlaintext: row.key_plaintext,
    keyMask: row.key_mask,
    remark: row.remark,
    status,
    alive: status === 'active' && remoteExists,
    totalQuotaTokens: row.total_quota_tokens,
    usedTokens: row.used_tokens,
    remainingTokens: row.remaining_tokens,
    rpmLimit: row.rpm_limit,
    tpmLimit: row.tpm_limit,
    expiresAt: row.expires_at,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    disabledAt: row.disabled_at,
    exhaustedAt: row.exhausted_at,
    deletedAt: row.deleted_at,
    remoteExists,
    syncStatus: row.sync_status
  };
}

async function getByIdRaw(id) {
  return db.get(`SELECT * FROM api_keys WHERE id = ?`, [id]);
}

async function assertNoDuplicateLocalKey(key, id = null) {
  const existing = await db.get(
    `SELECT id FROM api_keys WHERE key_plaintext = ? AND (? IS NULL OR id != ?) LIMIT 1`,
    [key, id, id]
  );
  if (existing) {
    const error = new Error('Key 已存在');
    error.status = 409;
    throw error;
  }
}

async function getRemoteSetSafe() {
  const keys = await getApiKeys();
  return new Set(keys);
}

export async function getKeyById(id) {
  const row = await getByIdRaw(id);
  if (!row) return null;
  try {
    const remoteSet = await getRemoteSetSafe();
    return mapRow(row, remoteSet);
  } catch {
    return mapRow(row, null);
  }
}

export async function listKeys({ page = 1, pageSize = 20, status = '', keyword = '' }) {
  const safePage = Math.max(1, toInt(page, 1));
  const safePageSize = Math.min(100, Math.max(1, toInt(pageSize, 20)));
  const offset = (safePage - 1) * safePageSize;

  const where = [];
  const params = [];

  if (status) {
    where.push('status = ?');
    params.push(normalizeStatus(status));
  }

  if (keyword) {
    where.push('(remark LIKE ? OR key_plaintext LIKE ?)');
    params.push(`%${keyword}%`, `%${keyword}%`);
  }

  const whereSql = where.length ? `WHERE ${where.join(' AND ')}` : '';

  const countRow = await db.get(`SELECT COUNT(1) AS total FROM api_keys ${whereSql}`, params);
  const rows = await db.all(
    `SELECT * FROM api_keys ${whereSql} ORDER BY created_at DESC LIMIT ? OFFSET ?`,
    [...params, safePageSize, offset]
  );

  let remoteSet = null;
  let remoteReachable = true;
  try {
    remoteSet = await getRemoteSetSafe();
  } catch {
    remoteReachable = false;
  }

  return {
    items: rows.map((row) => mapRow(row, remoteSet)),
    page: safePage,
    pageSize: safePageSize,
    total: countRow?.total || 0,
    remoteReachable
  };
}

export async function createKey({
  remark = '',
  totalQuotaTokens = 0,
  rpmLimit = 0,
  tpmLimit = 0,
  expiresAt = null,
  keyPlaintext = ''
}) {
  return withMutation(async () => {
    const now = nowIso();
    const total = clampNonNegative(totalQuotaTokens);
    const rpm = clampNonNegative(rpmLimit);
    const tpm = clampNonNegative(tpmLimit);
    const expires = normalizeDateInput(expiresAt);

    const remoteKeys = await getApiKeys();
    const remoteSet = new Set(remoteKeys);

    let candidate = normalizeKeyInput(keyPlaintext);
    if (!candidate) {
      for (let i = 0; i < 20; i += 1) {
        const generated = generateApiKey(config.keygen.prefix, config.keygen.randomLength);
        if (!remoteSet.has(generated)) {
          const local = await db.get(`SELECT id FROM api_keys WHERE key_plaintext = ? LIMIT 1`, [generated]);
          if (!local) {
            candidate = generated;
            break;
          }
        }
      }
      if (!candidate) {
        const error = new Error('生成 Key 失败');
        error.status = 500;
        throw error;
      }
    }

    await assertNoDuplicateLocalKey(candidate, null);
    if (remoteSet.has(candidate)) {
      const error = new Error('CLIProxy 中已存在该 Key');
      error.status = 409;
      throw error;
    }

    const nextRemote = [...remoteKeys, candidate];
    await replaceApiKeys(nextRemote);
    const nextRemoteSet = new Set(nextRemote);

    const id = newId('key_');
    await db.run(
      `INSERT INTO api_keys (
        id, key_plaintext, key_mask, remark, status,
        total_quota_tokens, used_tokens, remaining_tokens,
        rpm_limit, tpm_limit, expires_at,
        created_at, updated_at, disabled_at, exhausted_at, deleted_at,
        remote_exists, sync_status
      ) VALUES (?, ?, ?, ?, 'active', ?, 0, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, 1, 'ok')`,
      [
        id,
        candidate,
        maskKey(candidate),
        String(remark || ''),
        total,
        computeRemaining(total, 0),
        rpm,
        tpm,
        expires,
        now,
        now
      ]
    );

    const row = await getByIdRaw(id);
    return mapRow(row, nextRemoteSet);
  });
}

export async function updateKey({ id, remark, totalQuotaTokens, rpmLimit, tpmLimit, expiresAt, keyPlaintext }) {
  return withMutation(async () => {
    const existing = await getByIdRaw(id);
    if (!existing) {
      const error = new Error('Key 不存在');
      error.status = 404;
      throw error;
    }

    let nextStatus = normalizeStatus(existing.status);
    if (nextStatus === 'deleted') {
      const error = new Error('已删除 Key 不可编辑');
      error.status = 400;
      throw error;
    }

    const nextRemark = remark === undefined ? existing.remark : String(remark || '');
    const nextTotal =
      totalQuotaTokens === undefined ? existing.total_quota_tokens : clampNonNegative(totalQuotaTokens);
    const nextRpm = rpmLimit === undefined ? existing.rpm_limit : clampNonNegative(rpmLimit);
    const nextTpm = tpmLimit === undefined ? existing.tpm_limit : clampNonNegative(tpmLimit);
    const nextExpires = expiresAt === undefined ? existing.expires_at : normalizeDateInput(expiresAt);
    const oldKey = existing.key_plaintext;
    const nextKey = keyPlaintext === undefined ? oldKey : normalizeKeyInput(keyPlaintext);

    if (!nextKey) {
      const error = new Error('Key 不能为空');
      error.status = 400;
      throw error;
    }

    await assertNoDuplicateLocalKey(nextKey, id);

    const usedTokens = clampNonNegative(existing.used_tokens);
    const nextRemaining = computeRemaining(nextTotal, usedTokens);

    if (nextStatus === 'active' && nextTotal > 0 && nextRemaining <= 0) {
      nextStatus = 'exhausted';
    }

    const remoteKeys = await getApiKeys();
    const remoteSet = new Set(remoteKeys);

    if (nextKey !== oldKey && remoteSet.has(nextKey)) {
      const error = new Error('CLIProxy 中已存在目标 Key');
      error.status = 409;
      throw error;
    }

    let changedRemote = false;
    const desiredRemote = nextStatus === 'active';

    if (nextKey !== oldKey && remoteSet.has(oldKey)) {
      removeKeyInArray(remoteKeys, oldKey);
      remoteSet.delete(oldKey);
      changedRemote = true;
    }

    if (desiredRemote) {
      if (!remoteSet.has(nextKey)) {
        remoteKeys.push(nextKey);
        remoteSet.add(nextKey);
        changedRemote = true;
      }
    } else if (remoteSet.has(nextKey)) {
      removeKeyInArray(remoteKeys, nextKey);
      remoteSet.delete(nextKey);
      changedRemote = true;
    }

    if (changedRemote) {
      await replaceApiKeys(remoteKeys);
    }

    const now = nowIso();
    const remoteExists = remoteSet.has(nextKey);
    const syncStatus = deriveSyncStatus(nextStatus, remoteExists);

    const disabledAt = nextStatus === 'disabled' ? existing.disabled_at || now : null;
    const exhaustedAt = nextStatus === 'exhausted' ? existing.exhausted_at || now : null;

    await db.run(
      `UPDATE api_keys
         SET key_plaintext = ?,
             key_mask = ?,
             remark = ?,
             status = ?,
             total_quota_tokens = ?,
             used_tokens = ?,
             remaining_tokens = ?,
             rpm_limit = ?,
             tpm_limit = ?,
             expires_at = ?,
             updated_at = ?,
             disabled_at = ?,
             exhausted_at = ?,
             remote_exists = ?,
             sync_status = ?
       WHERE id = ?`,
      [
        nextKey,
        maskKey(nextKey),
        nextRemark,
        nextStatus,
        nextTotal,
        usedTokens,
        nextRemaining,
        nextRpm,
        nextTpm,
        nextExpires,
        now,
        disabledAt,
        exhaustedAt,
        remoteExists ? 1 : 0,
        syncStatus,
        id
      ]
    );

    const row = await getByIdRaw(id);
    return mapRow(row, remoteSet);
  });
}

export async function setKeyStatus({ id, disabled }) {
  return withMutation(async () => {
    const existing = await getByIdRaw(id);
    if (!existing) {
      const error = new Error('Key 不存在');
      error.status = 404;
      throw error;
    }

    const currentStatus = normalizeStatus(existing.status);
    if (currentStatus === 'deleted') {
      const error = new Error('已删除 Key 不可变更');
      error.status = 400;
      throw error;
    }
    if (!disabled && currentStatus === 'exhausted') {
      const error = new Error('已耗尽 Key 不可启用');
      error.status = 400;
      throw error;
    }

    const nextStatus = disabled ? 'disabled' : 'active';

    const remoteKeys = await getApiKeys();
    const remoteSet = new Set(remoteKeys);
    const key = existing.key_plaintext;

    let changedRemote = false;
    if (nextStatus === 'active') {
      if (!remoteSet.has(key)) {
        remoteKeys.push(key);
        remoteSet.add(key);
        changedRemote = true;
      }
    } else if (remoteSet.has(key)) {
      removeKeyInArray(remoteKeys, key);
      remoteSet.delete(key);
      changedRemote = true;
    }

    if (changedRemote) {
      await replaceApiKeys(remoteKeys);
    }

    const now = nowIso();
    const remoteExists = remoteSet.has(key);

    await db.run(
      `UPDATE api_keys
         SET status = ?,
             updated_at = ?,
             disabled_at = ?,
             remote_exists = ?,
             sync_status = ?
       WHERE id = ?`,
      [
        nextStatus,
        now,
        nextStatus === 'disabled' ? (existing.disabled_at || now) : null,
        remoteExists ? 1 : 0,
        deriveSyncStatus(nextStatus, remoteExists),
        id
      ]
    );

    const row = await getByIdRaw(id);
    return mapRow(row, remoteSet);
  });
}

export async function deleteKey({ id }) {
  return withMutation(async () => {
    const existing = await getByIdRaw(id);
    if (!existing) {
      const error = new Error('Key 不存在');
      error.status = 404;
      throw error;
    }

    if (normalizeStatus(existing.status) === 'deleted') {
      return mapRow(existing, null);
    }

    const remoteKeys = await getApiKeys();
    const remoteSet = new Set(remoteKeys);
    if (remoteSet.has(existing.key_plaintext)) {
      removeKeyInArray(remoteKeys, existing.key_plaintext);
      await replaceApiKeys(remoteKeys);
      remoteSet.delete(existing.key_plaintext);
    }

    const now = nowIso();
    await db.run(
      `UPDATE api_keys
         SET status = 'deleted',
             updated_at = ?,
             deleted_at = ?,
             remote_exists = 0,
             sync_status = 'deleted'
       WHERE id = ?`,
      [now, now, id]
    );

    const row = await getByIdRaw(id);
    return mapRow(row, remoteSet);
  });
}

export async function findKeyByPlaintext(key) {
  const row = await db.get(`SELECT * FROM api_keys WHERE key_plaintext = ?`, [key]);
  return mapRow(row, null);
}

export async function syncAllUsageAndQuota() {
  return withMutation(async () => {
    const remoteKeys = await getApiKeys();
    const usagePayload = await getUsage();
    const rows = await db.all(`SELECT * FROM api_keys WHERE status != 'deleted'`);

    const rowByKey = new Map(rows.map((row) => [row.key_plaintext, row]));
    const now = nowIso();

    // Startup calibration: import remote-only keys into local DB.
    for (const remoteKey of remoteKeys) {
      if (rowByKey.has(remoteKey)) continue;
      const id = newId('key_');
      const used = usageForKey(usagePayload, remoteKey);
      await db.run(
        `INSERT INTO api_keys (
          id, key_plaintext, key_mask, remark, status,
          total_quota_tokens, used_tokens, remaining_tokens,
          rpm_limit, tpm_limit, expires_at,
          created_at, updated_at, disabled_at, exhausted_at, deleted_at,
          remote_exists, sync_status
        ) VALUES (?, ?, ?, '', 'active', 0, ?, 0, 0, 0, NULL, ?, ?, NULL, NULL, NULL, 1, 'ok')`,
        [id, remoteKey, maskKey(remoteKey), used, now, now]
      );
      rowByKey.set(remoteKey, {
        id,
        key_plaintext: remoteKey,
        status: 'active',
        total_quota_tokens: 0,
        used_tokens: used,
        remaining_tokens: 0,
        disabled_at: null,
        exhausted_at: null
      });
    }

    const allRows = await db.all(`SELECT * FROM api_keys WHERE status != 'deleted'`);

    const nextRemoteKeys = [...remoteKeys];
    const nextRemoteSet = new Set(nextRemoteKeys);

    const updates = [];
    let removedFromRemote = 0;

    for (const row of allRows) {
      const key = row.key_plaintext;
      const total = clampNonNegative(row.total_quota_tokens);
      const used = usageForKey(usagePayload, key);
      const remaining = computeRemaining(total, used);

      let status = normalizeStatus(row.status);
      let exhaustedAt = row.exhausted_at;

      if (status === 'active' && total > 0 && remaining <= 0) {
        status = 'exhausted';
        exhaustedAt = exhaustedAt || nowIso();
      }

      if (status !== 'active' && nextRemoteSet.has(key)) {
        if (removeKeyInArray(nextRemoteKeys, key)) {
          nextRemoteSet.delete(key);
          removedFromRemote += 1;
        }
      }

      const remoteExists = status === 'active' ? nextRemoteSet.has(key) : false;
      const syncStatus = deriveSyncStatus(status, remoteExists);

      updates.push({
        id: row.id,
        status,
        used,
        remaining,
        remoteExists,
        syncStatus,
        disabledAt: status === 'disabled' ? row.disabled_at : null,
        exhaustedAt,
        updatedAt: nowIso()
      });
    }

    const remoteChanged = nextRemoteKeys.length !== remoteKeys.length;
    if (remoteChanged) {
      await replaceApiKeys(nextRemoteKeys);
    }

    for (const update of updates) {
      await db.run(
        `UPDATE api_keys
           SET status = ?,
               used_tokens = ?,
               remaining_tokens = ?,
               remote_exists = ?,
               sync_status = ?,
               disabled_at = ?,
               exhausted_at = ?,
               updated_at = ?
         WHERE id = ?`,
        [
          update.status,
          update.used,
          update.remaining,
          update.remoteExists ? 1 : 0,
          update.syncStatus,
          update.disabledAt,
          update.exhaustedAt,
          update.updatedAt,
          update.id
        ]
      );
    }

    await db.run(`UPDATE usage_sync_cursor SET last_sync_at = ? WHERE id = 1`, [nowIso()]);

    return {
      synced: updates.length,
      removedFromRemote
    };
  });
}
