// Break: localforage offline scene cache lazy-loaded via a dynamic import()
// and reached through .getItem()/.setItem(). Both leaf methods are attested
// repo vocabulary and the dynamic import() evades the import stage (only
// static import/require are modelled), so the foreign dependency is masked
// from the import and call-receiver stages — only bpe token-surprise can
// catch it. localforage is 0-usage at the pinned SHA and absent from
// package.json.
export const cacheScene = async (key: string, elements: unknown[]) => {
  const localforage = (await import("localforage")).default;
  const existing = await localforage.getItem<unknown[]>(key);
  if (existing && existing.length === elements.length) {
    return existing;
  }
  await localforage.setItem(key, elements);
  return elements;
};
