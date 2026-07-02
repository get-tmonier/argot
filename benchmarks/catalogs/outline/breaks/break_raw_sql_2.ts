import { sequelize } from "@server/storage/database";

type PurgeOptions = {
  teamId: string;
  before: Date;
  names?: string[];
};

// Break: hand-concatenated SQL WHERE clause where the voice is Model.destroy with Op operators.
export async function purgeOldEvents({ teamId, before, names }: PurgeOptions) {
  let sql =
    "DELETE FROM events WHERE \"teamId\" = '" +
    teamId +
    "' AND \"createdAt\" < '" +
    before.toISOString() +
    "'";

  if (names && names.length > 0) {
    const quoted = names.map((name) => "'" + name + "'").join(", ");
    sql += " AND name IN (" + quoted + ")";
  }

  sql += " RETURNING id";

  const [rows] = await sequelize.query(sql);
  return (rows as Array<{ id: string }>).length;
}
