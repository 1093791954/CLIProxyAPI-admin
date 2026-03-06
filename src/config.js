import path from 'node:path';
import process from 'node:process';
import dotenv from 'dotenv';

dotenv.config();

const rootDir = process.cwd();

function asInt(value, fallback) {
  const parsed = Number.parseInt(String(value ?? ''), 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function asNonEmpty(value, fallback) {
  const str = String(value ?? '').trim();
  return str || fallback;
}

function normalizeBaseUrl(value, fallback) {
  let raw = String(value ?? fallback ?? '').trim();
  if (!raw) return fallback;
  if (!/^https?:\/\//i.test(raw)) {
    raw = `http://${raw}`;
  }
  return raw.replace(/\/+$/, '');
}

const cliproxyBase = normalizeBaseUrl(process.env.CLIPROXY_BASE_URL, 'http://127.0.0.1:8317');

export const config = {
  app: {
    port: asInt(process.env.PORT, 8319),
    host: process.env.HOST || '0.0.0.0',
    jwtSecret: asNonEmpty(process.env.JWT_SECRET, 'change-me-in-production'),
    jwtExpiresIn: process.env.JWT_EXPIRES_IN || '12h',
    corsOrigin: process.env.CORS_ORIGIN || '*',
    trustProxy: process.env.TRUST_PROXY || 'loopback',
    publicRateLimitWindowSec: asInt(process.env.PUBLIC_RATE_LIMIT_WINDOW_SEC, 60),
    publicRateLimitMax: asInt(process.env.PUBLIC_RATE_LIMIT_MAX, 60),
    syncIntervalMs: asInt(process.env.SYNC_INTERVAL_MS, 60 * 1000)
  },
  db: {
    path: process.env.DB_PATH || path.join(rootDir, 'data', 'cliproxy-admin.db')
  },
  admin: {
    accessKey: asNonEmpty(process.env.ADMIN_ACCESS_KEY, 'ChangeMe-Cliproxy-Admin-Key')
  },
  cliproxy: {
    baseUrl: cliproxyBase,
    managementBaseUrl: `${cliproxyBase}/v0/management`,
    managementKey: process.env.CLIPROXY_MANAGEMENT_KEY || '',
    timeoutMs: asInt(process.env.CLIPROXY_TIMEOUT_MS, 15000)
  },
  keygen: {
    prefix: process.env.KEY_PREFIX || 'sk-',
    randomLength: Math.max(8, asInt(process.env.KEY_RANDOM_LENGTH, 32))
  }
};
