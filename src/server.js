import { createApp } from './app.js';
import { config } from './config.js';
import { startUsageSyncJob } from '../jobs/usageSyncJob.js';

const app = await createApp();

app.listen(config.app.port, config.app.host, () => {
  console.log(`CLIProxy Key Admin started: http://${config.app.host}:${config.app.port}`);
});

startUsageSyncJob();
