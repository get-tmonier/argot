# ID: server/utils/removeIndexCollision.ts:12
export default async function resolveIndexConflict(
  teamId: string,
  index: string,
  options: FindOptions = {}
) {
  const existing = await Collection.findOne({
    where: {
      teamId,
      deletedAt: null,
      index,
    },
    ...options,
  });

  if (!existing) {
    return index;
  }

  // Find the next collection ordered after this index to bisect against
  const following = await Collection.findAll({
    where: {
      teamId,
      deletedAt: null,
      index: Sequelize.literal(`"collection"."index" collate "C" > :index`),
    },
    attributes: ["id", "index"],
    limit: 1,
    order: [
      Sequelize.literal('"collection"."index" collate "C"'),
      ["updatedAt", "DESC"],
    ],
    replacements: { index },
    ...options,
  });

  const followingIndex = following.length ? following[0].index : null;
  return fractionalIndex(index, followingIndex);
}
