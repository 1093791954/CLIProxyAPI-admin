import { config } from './config.js';

const DEFAULT_TIMEOUT_MS = 60000;

export class CliproxyError extends Error {
  constructor(message, status = 500, payload = null) {
    super(message);
    this.name = 'CliproxyError';
    this.status = status;
    this.payload = payload;
  }
}

function withTimeout(signal, timeoutMs) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);

  if (signal) {
    signal.addEventListener('abort', () => controller.abort(), { once: true });
  }

  return {
    signal: controller.signal,
    cleanup: () => clearTimeout(timeout)
  };
}

function parsePayload(text) {
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return { raw: text };
  }
}

async function callCliproxy(path, { method = 'GET', body, signal, timeoutMs } = {}) {
  const url = `${config.cliproxy.managementBaseUrl}${path}`;
  const headers = { Accept: 'application/json' };

  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
  }

  if (config.cliproxy.managementKey) {
    headers.Authorization = `Bearer ${config.cliproxy.managementKey}`;
  }

  const timed = withTimeout(signal, timeoutMs || config.cliproxy.timeoutMs || DEFAULT_TIMEOUT_MS);

  let response;
  try {
    response = await fetch(url, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: timed.signal
    });
  } catch (error) {
    timed.cleanup();
    throw new CliproxyError(`CLIProxyAPI 请求失败: ${error.message}`, 502);
  }
  timed.cleanup();

  const text = await response.text();
  const payload = parsePayload(text);

  if (!response.ok) {
    throw new CliproxyError(`CLIProxyAPI 返回异常 (${response.status})`, response.status, payload);
  }

  return payload;
}

function normalizeKeyList(payload) {
  const list = payload?.['api-keys'] ?? payload?.apiKeys ?? payload;
  if (!Array.isArray(list)) return [];
  return list.map((item) => String(item ?? '').trim()).filter(Boolean);
}

export async function getApiKeys() {
  const payload = await callCliproxy('/api-keys');
  return normalizeKeyList(payload);
}

export async function replaceApiKeys(keys) {
  const normalized = [...new Set((keys || []).map((item) => String(item ?? '').trim()).filter(Boolean))];
  await callCliproxy('/api-keys', {
    method: 'PUT',
    body: normalized
  });
  return normalized;
}

export async function patchApiKey(index, value) {
  return callCliproxy('/api-keys', {
    method: 'PATCH',
    body: { index, value }
  });
}

export async function deleteApiKey(index) {
  return callCliproxy(`/api-keys?index=${encodeURIComponent(index)}`, {
    method: 'DELETE'
  });
}

export async function getUsage() {
  return callCliproxy('/usage', {
    timeoutMs: Math.max(config.cliproxy.timeoutMs, 60000)
  });
}

export async function getConfig() {
  return callCliproxy('/config');
}
