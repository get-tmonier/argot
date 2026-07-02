import { Pool } from "pg";

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

// Break: direct pg Pool with interpolated SQL strings where the data layer voice is Sequelize models.
export async function findRecentlyViewedDocuments(
  userId: string,
  teamId: string
) {
  const client = await pool.connect();
  try {
    const result = await client.query(
      `SELECT d.id, d.title, v."updatedAt" AS "viewedAt"
       FROM documents d
       JOIN views v ON v."documentId" = d.id
       WHERE v."userId" = '${userId}'
         AND d."teamId" = '${teamId}'
         AND d."deletedAt" IS NULL
       ORDER BY v."updatedAt" DESC
       LIMIT 20`
    );
    return result.rows;
  } finally {
    client.release();
  }
}
