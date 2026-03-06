export function errorHandler(err, req, res, next) {
  const status = err.status || 500;
  const message = err.message || '服务器内部错误';
  const extra = err.payload ? { payload: err.payload } : undefined;

  if (status >= 500) {
    console.error('[ERROR]', err);
  }

  res.status(status).json({ error: message, ...extra });
}
