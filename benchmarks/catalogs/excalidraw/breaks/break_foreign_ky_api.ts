// Break: ky HTTP client lazy-loaded via a dynamic import() and reached
// through .get(...).json(). Both leaf methods are attested repo vocabulary and
// the dynamic import() evades the import stage (only static import/require are
// modelled), so the foreign HTTP client is masked from the import and
// call-receiver stages — only bpe token-surprise can catch it. ky is 0-usage
// at the pinned SHA and absent from package.json.
export const fetchSharedScene = async (id: string) => {
  const { default: ky } = await import("ky");
  const response = await ky.get(`https://api.excalidraw.example.com/scenes/${id}`);
  const scene = await response.json();
  return scene;
};
