import { db } from '../db.js';
import { newId, nowIso } from '../utils.js';

export async function writeAuditLog({ actor, action, targetKeyId = null, payload = {} }) {
  await db.run(
    `INSERT INTO audit_logs (id, actor, action, target_key_id, payload_json, created_at)
     VALUES (?, ?, ?, ?, ?, ?)`,
    [newId('audit_'), actor, action, targetKeyId, JSON.stringify(payload), nowIso()]
  );
}

export async function listAuditLogs({ limit = 100, offset = 0 }) {
  return db.all(
    `SELECT id, actor, action, target_key_id AS targetKeyId, payload_json AS payloadJson, created_at AS createdAt
     FROM audit_logs
     ORDER BY created_at DESC
     LIMIT ? OFFSET ?`,
    [limit, offset]
  );
}
