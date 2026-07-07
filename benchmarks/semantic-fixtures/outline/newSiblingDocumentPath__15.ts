# ID: app/utils/routeHelpers.ts:131
export function siblingDocumentUrl(params: {
  collectionId?: string | null;
  parentDocumentId?: string;
  index: number;
}): string {
  const query: Record<string, string> = {
    index: String(params.index),
  };
  if (params.parentDocumentId) {
    query.parentDocumentId = params.parentDocumentId;
  }
  if (params.collectionId) {
    query.collectionId = params.collectionId;
  }

  return `/doc/new?${queryString.stringify(query)}`;
}
