import jwt from 'jsonwebtoken';
import { config } from './config.js';

export function signAdminToken(payload) {
  return jwt.sign(payload, config.app.jwtSecret, {
    expiresIn: config.app.jwtExpiresIn
  });
}

export function verifyAdminToken(token) {
  return jwt.verify(token, config.app.jwtSecret);
}
