import { Router } from 'express';
import { createKey, deleteKey, getKeyById, listKeys, setKeyStatus, updateKey } from '../src/services/keyService.js';
import { writeAuditLog } from '../src/services/auditService.js';

export const adminKeysRouter = Router();

adminKeysRouter.get('/', async (req, res, next) => {
  try {
    const data = await listKeys({
      page: req.query.page,
      pageSize: req.query.pageSize,
      status: req.query.status,
      keyword: req.query.keyword
    });
    return res.json(data);
  } catch (error) {
    return next(error);
  }
});

adminKeysRouter.get('/:id', async (req, res, next) => {
  try {
    const row = await getKeyById(req.params.id);
    if (!row) return res.status(404).json({ error: 'Key 不存在' });
    return res.json(row);
  } catch (error) {
    return next(error);
  }
});

adminKeysRouter.post('/', async (req, res, next) => {
  try {
    const {
      remark = '',
      totalQuotaTokens = 0,
      rpmLimit = 0,
      tpmLimit = 0,
      expiresAt = null,
      keyPlaintext = ''
    } = req.body || {};

    const created = await createKey({
      remark,
      totalQuotaTokens,
      rpmLimit,
      tpmLimit,
      expiresAt,
      keyPlaintext
    });

    await writeAuditLog({
      actor: req.admin.username,
      action: 'key.create',
      targetKeyId: created.id,
      payload: {
        remark,
        totalQuotaTokens,
        rpmLimit,
        tpmLimit,
        expiresAt,
        keyPlaintext: created.keyPlaintext
      }
    });

    return res.status(201).json(created);
  } catch (error) {
    return next(error);
  }
});

adminKeysRouter.patch('/:id', async (req, res, next) => {
  try {
    const updated = await updateKey({
      id: req.params.id,
      remark: req.body?.remark,
      totalQuotaTokens: req.body?.totalQuotaTokens,
      rpmLimit: req.body?.rpmLimit,
      tpmLimit: req.body?.tpmLimit,
      expiresAt: req.body?.expiresAt,
      keyPlaintext: req.body?.keyPlaintext
    });

    await writeAuditLog({
      actor: req.admin.username,
      action: 'key.update',
      targetKeyId: updated.id,
      payload: req.body || {}
    });

    return res.json(updated);
  } catch (error) {
    return next(error);
  }
});

adminKeysRouter.post('/:id/disable', async (req, res, next) => {
  try {
    const updated = await setKeyStatus({ id: req.params.id, disabled: true });
    await writeAuditLog({
      actor: req.admin.username,
      action: 'key.disable',
      targetKeyId: updated.id,
      payload: {}
    });
    return res.json(updated);
  } catch (error) {
    return next(error);
  }
});

adminKeysRouter.post('/:id/enable', async (req, res, next) => {
  try {
    const updated = await setKeyStatus({ id: req.params.id, disabled: false });
    await writeAuditLog({
      actor: req.admin.username,
      action: 'key.enable',
      targetKeyId: updated.id,
      payload: {}
    });
    return res.json(updated);
  } catch (error) {
    return next(error);
  }
});

adminKeysRouter.delete('/:id', async (req, res, next) => {
  try {
    const updated = await deleteKey({ id: req.params.id });
    await writeAuditLog({
      actor: req.admin.username,
      action: 'key.delete',
      targetKeyId: updated.id,
      payload: {}
    });
    return res.json(updated);
  } catch (error) {
    return next(error);
  }
});
