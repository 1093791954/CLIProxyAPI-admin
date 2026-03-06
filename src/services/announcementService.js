import { db } from '../db.js';
import { newId, nowIso } from '../utils.js';

function map(row) {
  if (!row) return null;
  return {
    id: row.id,
    title: row.title,
    contentMd: row.content_md,
    isCurrent: Boolean(row.is_current),
    version: row.version,
    createdAt: row.created_at,
    createdBy: row.created_by
  };
}

export async function listAnnouncements() {
  const rows = await db.all(`SELECT * FROM announcements ORDER BY version DESC`);
  return rows.map(map);
}

export async function getCurrentAnnouncement() {
  const row = await db.get(`SELECT * FROM announcements WHERE is_current = 1 ORDER BY version DESC LIMIT 1`);
  return map(row);
}

export async function createAnnouncement({ title, contentMd, createdBy }) {
  const row = await db.get(`SELECT MAX(version) AS maxVersion FROM announcements`);
  const version = (row?.maxVersion || 0) + 1;
  const id = newId('ann_');
  await db.run(
    `INSERT INTO announcements (id, title, content_md, is_current, version, created_at, created_by)
     VALUES (?, ?, ?, 0, ?, ?, ?)`,
    [id, title, contentMd, version, nowIso(), createdBy]
  );
  return db.get(`SELECT * FROM announcements WHERE id = ?`, [id]).then(map);
}

export async function publishAnnouncement(id) {
  await db.tx(async (tx) => {
    await tx.run(`UPDATE announcements SET is_current = 0 WHERE is_current = 1`);
    const updated = await tx.run(`UPDATE announcements SET is_current = 1 WHERE id = ?`, [id]);
    if (updated.changes === 0) {
      const error = new Error('公告不存在');
      error.status = 404;
      throw error;
    }
  });

  return getCurrentAnnouncement();
}
