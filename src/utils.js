import { randomBytes, randomUUID, timingSafeEqual } from 'node:crypto';

const KEY_CHARS = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';

export function nowIso() {
  return new Date().toISOString();
}

export function maskKey(key) {
  if (!key || key.length < 8) return '****';
  return `${key.slice(0, 4)}...${key.slice(-4)}`;
}

export function safeJsonParse(raw, fallback = null) {
  try {
    return JSON.parse(raw);
  } catch {
    return fallback;
  }
}

export function pick(obj, keys) {
  const out = {};
  for (const key of keys) {
    if (obj[key] !== undefined) out[key] = obj[key];
  }
  return out;
}

export function newId(prefix = '') {
  return `${prefix}${randomUUID()}`;
}

export function toInt(value, fallback = 0) {
  const n = Number.parseInt(String(value ?? ''), 10);
  return Number.isFinite(n) ? n : fallback;
}

export function clampNonNegative(value) {
  return Math.max(0, toInt(value, 0));
}

export function safeEqual(left, right) {
  const leftBuf = Buffer.from(String(left ?? ''));
  const rightBuf = Buffer.from(String(right ?? ''));
  if (leftBuf.length !== rightBuf.length) return false;
  return timingSafeEqual(leftBuf, rightBuf);
}

export function generateApiKey(prefix = 'sk-', randomLength = 32) {
  const length = Math.max(8, toInt(randomLength, 32));
  let randomPart = '';
  while (randomPart.length < length) {
    const buf = randomBytes(length);
    for (const value of buf) {
      randomPart += KEY_CHARS[value % KEY_CHARS.length];
      if (randomPart.length >= length) break;
    }
  }
  return `${prefix}${randomPart}`;
}
