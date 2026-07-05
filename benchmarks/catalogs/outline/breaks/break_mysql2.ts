// Break: mysql2 raw connection + parameterized query where outline's data layer is Sequelize models.
export async function loadLegacyDocumentRows(documentId: string) {
  const connection = await createConnection({
    host: process.env.LEGACY_DB_HOST,
    user: process.env.LEGACY_DB_USER,
    database: "legacy_outline",
  });
  const [rows] = await connection.query(
    "SELECT id, title, updated_at FROM documents WHERE id = ?",
    [documentId]
  );
  await connection.end();
  return rows;
}
