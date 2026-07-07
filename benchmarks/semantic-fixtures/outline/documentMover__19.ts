# ID: server/commands/documentMover.ts:23
async function relocateDocument(
  ctx: APIContext,
  { document, collectionId, parentDocumentId = null, index }: Props
): Promise<Result> {
  const { user } = ctx.state.auth;
  const { transaction } = ctx.state;

  const collectionChanged = collectionId !== document.collectionId;
  const previousCollectionId = document.collectionId;
  const result: Result = {
    collections: [],
    documents: [],
    collectionChanged,
  };

  // Load and lock the source collection (and destination if it differs)
  const collection = await Collection.findByPk(document.collectionId!, {
    includeDocumentStructure: true,
    transaction,
    lock: Transaction.LOCK.UPDATE,
    paranoid: false,
  });

  let newCollection = collection;
  if (collectionChanged && collectionId) {
    newCollection = await Collection.findByPk(collectionId, {
      includeDocumentStructure: true,
      transaction,
      lock: Transaction.LOCK.UPDATE,
    });
  } else if (!collectionId) {
    newCollection = null;
  }

  if (document.publishedAt) {
    const response = await collection?.removeDocumentInStructure(document, {
      transaction,
      save: collectionChanged,
    });

    let documentJson = response?.[0];
    const fromIndex = response?.[1] || 0;

    if (!documentJson) {
      documentJson = await document.toNavigationNode({ transaction });
    }

    // Removing the item above shrinks the list, so compensate when we are
    // reordering within the same parent and moving further down.
    const sameParent =
      document.parentDocumentId === parentDocumentId &&
      document.collectionId === collectionId;
    const toIndex =
      index !== undefined && sameParent && fromIndex < index
        ? index - 1
        : index;

    document.collectionId = collectionId;
    document.parentDocumentId = parentDocumentId;
    document.lastModifiedById = user.id;
    document.updatedBy = user;

    if (newCollection) {
      await newCollection.addDocumentToStructure(document, toIndex, {
        documentJson,
        transaction,
      });
    }
  } else {
    document.collectionId = collectionId;
    document.parentDocumentId = parentDocumentId;
    document.lastModifiedById = user.id;
    document.updatedBy = user;
  }

  if (collection && document.publishedAt) {
    result.collections.push(collection);
  }

  // When the collection changes, propagate the new collectionId to all
  // descendant documents in a single update.
  if (collectionChanged) {
    const childDocumentIds = await document.findAllChildDocumentIds();

    if (collectionId) {
      newCollection = await Collection.findByPk(collectionId, {
        userId: user.id,
        includeDocumentStructure: true,
        rejectOnEmpty: true,
        transaction,
      });

      result.collections.push(newCollection);

      await Document.update(
        { collectionId: newCollection.id },
        { transaction, where: { id: childDocumentIds } }
      );
    } else {
      // Moving to drafts: unpublish and reparent the children upward
      document.publishedAt = null;

      await Document.update(
        { parentDocumentId: document.parentDocumentId },
        { transaction, where: { id: childDocumentIds } }
      );
    }

    const documents = await Document.findAll({
      where: { id: childDocumentIds },
      transaction,
    });

    document.collection = newCollection;
    result.documents.push(
      ...documents.map((doc) => {
        if (newCollection) {
          doc.collection = newCollection;
        }
        return doc;
      })
    );

    // Drop any pin tied to the old collection to avoid a cross-collection pin
    const pin = await Pin.findOne({
      where: {
        documentId: document.id,
        collectionId: previousCollectionId,
      },
      transaction,
      lock: Transaction.LOCK.UPDATE,
    });

    await pin?.destroyWithCtx(ctx);
  }

  result.documents.push(document);

  await document.saveWithCtx(ctx, undefined, {
    name: "move",
    data: {
      collectionIds: result.collections.map((c) => c.id),
      documentIds: result.documents.map((d) => d.id),
    },
  });

  return result;
}
