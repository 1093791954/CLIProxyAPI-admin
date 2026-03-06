import { config } from '../src/config.js';
import { syncAllUsageAndQuota } from '../src/services/keyService.js';

let timer = null;
let running = false;

export function startUsageSyncJob() {
  if (timer) return timer;

  async function runOnce() {
    if (running) return;
    running = true;
    try {
      await syncAllUsageAndQuota();
    } catch (error) {
      console.error('[SYNC] Usage sync failed:', error.message);
    } finally {
      running = false;
    }
  }

  runOnce();
  timer = setInterval(runOnce, config.app.syncIntervalMs);
  return timer;
}

export function stopUsageSyncJob() {
  if (!timer) return;
  clearInterval(timer);
  timer = null;
}
