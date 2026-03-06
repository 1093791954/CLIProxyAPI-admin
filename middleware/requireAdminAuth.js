import { verifyAdminToken } from '../src/auth.js';
import { getAdminById } from '../src/services/adminService.js';

export async function requireAdminAuth(req, res, next) {
  try {
    const authHeader = req.headers.authorization || '';
    const token = authHeader.startsWith('Bearer ') ? authHeader.slice(7) : '';
    if (!token) {
      return res.status(401).json({ error: '未登录' });
    }

    const decoded = verifyAdminToken(token);
    const admin = await getAdminById(decoded.sub);
    if (!admin) {
      return res.status(401).json({ error: '账号不存在' });
    }

    req.admin = {
      id: admin.id,
      username: admin.username,
      mustChangePassword: Boolean(admin.mustChangePassword)
    };

    return next();
  } catch (error) {
    return res.status(401).json({ error: '登录状态失效' });
  }
}
