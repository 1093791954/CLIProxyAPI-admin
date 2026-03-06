import { Router } from 'express';
import { loginAdmin } from '../src/services/adminService.js';

export const adminAuthRouter = Router();

adminAuthRouter.post('/login', async (req, res, next) => {
  try {
    const { accessKey = '' } = req.body || {};
    if (!accessKey) {
      return res.status(400).json({ error: '管理秘钥必填' });
    }

    const session = await loginAdmin({ accessKey });
    if (!session) {
      return res.status(401).json({ error: '管理秘钥错误' });
    }

    return res.json(session);
  } catch (error) {
    return next(error);
  }
});
