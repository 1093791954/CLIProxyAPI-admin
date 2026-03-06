import { Router } from 'express';
import {
  createAnnouncement,
  getCurrentAnnouncement,
  listAnnouncements,
  publishAnnouncement
} from '../src/services/announcementService.js';
import { writeAuditLog } from '../src/services/auditService.js';

export const announcementRouter = Router();

announcementRouter.get('/current', async (req, res, next) => {
  try {
    const current = await getCurrentAnnouncement();
    return res.json(current || null);
  } catch (error) {
    return next(error);
  }
});

announcementRouter.get('/', async (req, res, next) => {
  try {
    const list = await listAnnouncements();
    return res.json(list);
  } catch (error) {
    return next(error);
  }
});

announcementRouter.post('/', async (req, res, next) => {
  try {
    const { title = '', contentMd = '' } = req.body || {};
    if (!title || !contentMd) {
      return res.status(400).json({ error: '标题和内容必填' });
    }

    const created = await createAnnouncement({
      title,
      contentMd,
      createdBy: req.admin.username
    });

    await writeAuditLog({
      actor: req.admin.username,
      action: 'announcement.create',
      payload: { id: created.id, title: created.title, version: created.version }
    });

    return res.status(201).json(created);
  } catch (error) {
    return next(error);
  }
});

announcementRouter.post('/:id/publish', async (req, res, next) => {
  try {
    const current = await publishAnnouncement(req.params.id);
    await writeAuditLog({
      actor: req.admin.username,
      action: 'announcement.publish',
      payload: { id: req.params.id }
    });

    return res.json(current);
  } catch (error) {
    return next(error);
  }
});
