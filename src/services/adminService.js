import { signAdminToken } from '../auth.js';
import { config } from '../config.js';
import { safeEqual } from '../utils.js';

const ADMIN_ID = 'admin_static';
const ADMIN_USERNAME = 'admin';

export async function ensureBootstrapAdmin() {
  return null;
}

export async function loginAdmin({ accessKey }) {
  if (!safeEqual(String(accessKey || ''), String(config.admin.accessKey || ''))) {
    return null;
  }

  const accessToken = signAdminToken({
    sub: ADMIN_ID,
    username: ADMIN_USERNAME,
    role: 'admin'
  });

  return {
    accessToken,
    expiresIn: config.app.jwtExpiresIn,
    user: {
      id: ADMIN_ID,
      username: ADMIN_USERNAME
    }
  };
}

export async function getAdminById(id) {
  if (id !== ADMIN_ID) return null;
  return {
    id: ADMIN_ID,
    username: ADMIN_USERNAME,
    mustChangePassword: false
  };
}
