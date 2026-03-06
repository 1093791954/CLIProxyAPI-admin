import { config } from '../src/config.js';

const bucket = new Map();

function cleanupExpired(windowMs, currentTs) {
  for (const [key, value] of bucket.entries()) {
    if (currentTs - value.start > windowMs * 2) {
      bucket.delete(key);
    }
  }
}

export function publicRateLimit(req, res, next) {
  const ip = String(req.ip || req.headers['x-forwarded-for'] || req.socket.remoteAddress || 'unknown')
    .split(',')[0]
    .trim();

  const windowMs = config.app.publicRateLimitWindowSec * 1000;
  const max = config.app.publicRateLimitMax;
  const ts = Date.now();

  const hit = bucket.get(ip);
  if (!hit || ts - hit.start > windowMs) {
    bucket.set(ip, { start: ts, count: 1 });
    cleanupExpired(windowMs, ts);
    return next();
  }

  hit.count += 1;
  if (hit.count > max) {
    return res.status(503).json({ error: '服务器繁忙，请稍后再试' });
  }

  return next();
}
