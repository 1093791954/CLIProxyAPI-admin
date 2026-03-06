import { config } from './config.js';

class LiteLLMError extends Error {
  constructor(message, status = 500, payload = null) {
    super(message);
    this.name = 'LiteLLMError';
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

async function callLiteLLM(path, { method = 'GET', body, signal } = {}) {
  const url = `${config.litellm.baseUrl}${path}`;
  const headers = {
    'Content-Type': 'application/json'
  };

  if (config.litellm.masterKey) {
    headers.Authorization = `Bearer ${config.litellm.masterKey}`;
    headers['x-litellm-api-key'] = config.litellm.masterKey;
  }

  const timed = withTimeout(signal, config.litellm.timeoutMs);
  let res;
  try {
    res = await fetch(url, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
      signal: timed.signal
    });
  } catch (error) {
    timed.cleanup();
    throw new LiteLLMError(`LiteLLM 请求失败: ${error.message}`, 502);
  }
  timed.cleanup();

  let payload = null;
  const text = await res.text();
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      payload = { raw: text };
    }
  }

  if (!res.ok) {
    throw new LiteLLMError(`LiteLLM 返回错误(${res.status})`, res.status, payload);
  }

  return payload;
}

function extractKeyFromCreateResponse(payload) {
  if (!payload || typeof payload !== 'object') return null;
  if (typeof payload.key === 'string' && payload.key) return payload.key;
  if (typeof payload.token === 'string' && payload.token) return payload.token;
  if (payload.data && typeof payload.data.key === 'string') return payload.data.key;
  return null;
}

function extractLiteKeyId(payload) {
  if (!payload || typeof payload !== 'object') return null;
  if (typeof payload.key_id === 'string') return payload.key_id;
  if (payload.data && typeof payload.data.key_id === 'string') return payload.data.key_id;
  return null;
}

export async function createRemoteKey({ remark, rpmLimit, tpmLimit, expiresAt }) {
  const body = {
    models: ['*'],
    key_alias: remark || undefined,
    metadata: {
      remark: remark || ''
    },
    rpm_limit: rpmLimit,
    tpm_limit: tpmLimit,
    expires: expiresAt || undefined
  };

  const payload = await callLiteLLM('/key/generate', {
    method: 'POST',
    body
  });

  const key = extractKeyFromCreateResponse(payload);
  if (!key) {
    throw new LiteLLMError('LiteLLM 创建 key 成功但未返回明文 key', 500, payload);
  }

  return {
    key,
    litellmKeyId: extractLiteKeyId(payload),
    raw: payload
  };
}

export async function updateRemoteKey({ key, remark, rpmLimit, tpmLimit, expiresAt }) {
  const body = {
    key,
    key_alias: remark,
    metadata: {
      remark: remark || ''
    },
    rpm_limit: rpmLimit,
    tpm_limit: tpmLimit,
    expires: expiresAt || null
  };

  return callLiteLLM('/key/update', {
    method: 'POST',
    body
  });
}

export async function setRemoteKeyDisabled({ key, disabled }) {
  const body = {
    key,
    blocked: Boolean(disabled)
  };

  return callLiteLLM('/key/update', {
    method: 'POST',
    body
  });
}

export async function getRemoteKeyInfo({ key }) {
  const encoded = encodeURIComponent(key);
  return callLiteLLM(`/key/info?key=${encoded}`);
}

export async function listRemoteKeys() {
  return callLiteLLM('/key/list', {
    method: 'GET'
  });
}

export function extractUsageTokens(infoPayload) {
  if (!infoPayload || typeof infoPayload !== 'object') return 0;

  const directCandidates = [
    infoPayload.total_tokens,
    infoPayload.usage_tokens,
    infoPayload.used_tokens,
    infoPayload.token_usage
  ];

  for (const candidate of directCandidates) {
    const n = Number(candidate);
    if (Number.isFinite(n) && n >= 0) return Math.floor(n);
  }

  const nested = infoPayload.info || infoPayload.key_info || infoPayload.data || null;
  if (nested && typeof nested === 'object') {
    const nestedCandidates = [
      nested.total_tokens,
      nested.usage_tokens,
      nested.used_tokens,
      nested.token_usage
    ];
    for (const candidate of nestedCandidates) {
      const n = Number(candidate);
      if (Number.isFinite(n) && n >= 0) return Math.floor(n);
    }
  }

  return 0;
}

export { LiteLLMError };
