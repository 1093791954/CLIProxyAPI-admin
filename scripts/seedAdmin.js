import { migrate } from '../src/migrate.js';
import { ensureBootstrapAdmin } from '../src/services/adminService.js';
import { config } from '../src/config.js';

await migrate();
await ensureBootstrapAdmin();

console.log('管理员已初始化');
console.log(`用户名: ${config.adminBootstrap.username}`);
console.log(`初始密码: ${config.adminBootstrap.password}`);
