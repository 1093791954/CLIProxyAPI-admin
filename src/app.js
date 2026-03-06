import cors from 'cors';
import express from 'express';
import morgan from 'morgan';
import path from 'node:path';

import { config } from './config.js';
import { migrate } from './migrate.js';
import { ensureBootstrapAdmin } from './services/adminService.js';

import { adminAuthRouter } from '../routes/adminAuth.js';
import { adminKeysRouter } from '../routes/adminKeys.js';
import { announcementRouter } from '../routes/announcements.js';
import { auditRouter } from '../routes/audit.js';
import { publicRouter } from '../routes/public.js';

import { requireAdminAuth } from '../middleware/requireAdminAuth.js';
import { errorHandler } from '../middleware/errorHandler.js';

const publicDir = path.join(process.cwd(), 'public');

export async function createApp() {
  await migrate();
  await ensureBootstrapAdmin();

  const app = express();

  app.set('trust proxy', config.app.trustProxy);
  app.use(cors({ origin: config.app.corsOrigin }));
  app.use(express.json({ limit: '1mb' }));
  app.use(morgan('combined'));

  app.use('/api/admin', (req, res, next) => {
    res.set('Cache-Control', 'no-store, no-cache, must-revalidate, proxy-revalidate');
    res.set('Pragma', 'no-cache');
    res.set('Expires', '0');
    delete req.headers['if-none-match'];
    delete req.headers['if-modified-since'];
    next();
  });

  app.get('/api/health', (req, res) => {
    res.json({ ok: true, ts: new Date().toISOString() });
  });

  app.use('/api/admin/auth', adminAuthRouter);
  app.use('/api/admin/keys', requireAdminAuth, adminKeysRouter);
  app.use('/api/admin/announcements', requireAdminAuth, announcementRouter);
  app.use('/api/admin/audit', requireAdminAuth, auditRouter);

  app.use('/api/public', publicRouter);

  app.get('/', (req, res) => {
    res.status(204).end();
  });

  app.use(express.static(publicDir, { index: false }));

  app.get('/check', (req, res) => {
    res.sendFile(path.join(publicDir, 'check.html'));
  });

  app.get('/admin', (req, res) => {
    res.sendFile(path.join(publicDir, 'admin-login.html'));
  });

  app.get('/admin/dashboard', (req, res) => {
    res.sendFile(path.join(publicDir, 'admin.html'));
  });

  app.get('/admin/login', (req, res) => {
    res.sendFile(path.join(publicDir, 'admin-login.html'));
  });

  app.use(errorHandler);

  return app;
}
