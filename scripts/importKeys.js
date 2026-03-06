import { migrate } from '../src/migrate.js';
import { importRemoteKeys } from '../src/services/keyService.js';

await migrate();
const result = await importRemoteKeys();
console.log('导入完成:', result);
