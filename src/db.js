import fs from 'node:fs';
import path from 'node:path';
import Database from 'better-sqlite3';
import { config } from './config.js';

const dbFile = config.db.path;
const dbDir = path.dirname(dbFile);
if (!fs.existsSync(dbDir)) {
  fs.mkdirSync(dbDir, { recursive: true });
}

const raw = new Database(dbFile);
raw.pragma('journal_mode = WAL');
raw.pragma('foreign_keys = ON');

function run(sql, params = []) {
  const stmt = raw.prepare(sql);
  const info = stmt.run(params);
  return Promise.resolve({
    lastID: Number(info.lastInsertRowid || 0),
    changes: info.changes || 0
  });
}

function get(sql, params = []) {
  const stmt = raw.prepare(sql);
  const row = stmt.get(params);
  return Promise.resolve(row || null);
}

function all(sql, params = []) {
  const stmt = raw.prepare(sql);
  const rows = stmt.all(params);
  return Promise.resolve(rows || []);
}

async function tx(handler) {
  raw.exec('BEGIN');
  try {
    const result = await handler({ run, get, all });
    raw.exec('COMMIT');
    return result;
  } catch (error) {
    raw.exec('ROLLBACK');
    throw error;
  }
}

export const db = { run, get, all, tx, raw };
