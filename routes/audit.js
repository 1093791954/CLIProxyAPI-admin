import { Router } from 'express';
import { listAuditLogs } from '../src/services/auditService.js';

export const auditRouter = Router();

auditRouter.get('/', async (req, res, next) => {
  try {
    const limit = Number.parseInt(String(req.query.limit || '100'), 10);
    const offset = Number.parseInt(String(req.query.offset || '0'), 10);
    const logs = await listAuditLogs({
      limit: Number.isFinite(limit) ? Math.min(500, Math.max(1, limit)) : 100,
      offset: Number.isFinite(offset) ? Math.max(0, offset) : 0
    });
    return res.json(logs);
  } catch (error) {
    return next(error);
  }
});
