import { migrate } from '../src/migrate.js';

await migrate();
console.log('数据库迁移完成');
