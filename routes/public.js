import { Router } from 'express';
import { config } from '../src/config.js';
import { getCurrentAnnouncement } from '../src/services/announcementService.js';
import { findKeyByPlaintext } from '../src/services/keyService.js';
import { publicRateLimit } from '../middleware/publicRateLimit.js';

export const publicRouter = Router();

function buildUsageGuide(req) {
  const base = new URL(config.cliproxy.baseUrl);
  const proto = String(req.headers['x-forwarded-proto'] || req.protocol || base.protocol.replace(':', ''))
    .split(',')[0]
    .trim();
  const host = req.hostname || base.hostname;
  const port = base.port ? `:${base.port}` : '';
  return {
    baseUrl: `${proto}://${host}${port}/v1`,
    headerName: 'Authorization',
    headerFormat: 'Bearer <YOUR_KEY>'
  };
}

publicRouter.get('/announcement/current', async (req, res, next) => {
  try {
    const current = await getCurrentAnnouncement();
    return res.json(current || null);
  } catch (error) {
    return next(error);
  }
});

publicRouter.post('/key/check', publicRateLimit, async (req, res, next) => {
  try {
    const key = String(req.body?.key || '').trim();
    if (!key) {
      return res.status(400).json({ error: '请输入 Key' });
    }

    const row = await findKeyByPlaintext(key);
    const announcement = await getCurrentAnnouncement();
    const usageGuide = buildUsageGuide(req);

    if (!row) {
      return res.status(404).json({
        alive: false,
        status: 'missing',
        message: 'Key 不存在或已失效',
        announcement,
        usageGuide
      });
    }

    return res.json({
      alive: Boolean(row.alive),
      status: row.status,
      totalQuotaTokens: row.totalQuotaTokens,
      usedTokens: row.usedTokens,
      remainingTokens: row.remainingTokens,
      expiresAt: row.expiresAt,
      remoteExists: row.remoteExists,
      syncStatus: row.syncStatus,
      quotaUnlimited: row.totalQuotaTokens <= 0,
      announcement,
      usageGuide
    });
  } catch (error) {
    return next(error);
  }
});
