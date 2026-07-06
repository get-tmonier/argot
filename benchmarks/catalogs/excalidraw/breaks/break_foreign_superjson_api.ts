// Break: superjson serializer lazy-loaded via a dynamic import() and reached
// through .stringify()/.parse(), mimicking the repo's own JSON idiom. Both
// leaf methods are attested repo vocabulary and the dynamic import() evades
// the import stage (only static import/require are modelled), so the foreign
// serializer is masked from the import and call-receiver stages — only bpe
// token-surprise can catch it. superjson is 0-usage at the pinned SHA and
// absent from package.json.
export const encodeScenePayload = async (scene: { elements: unknown[] }) => {
  const superjson = (await import("superjson")).default;
  const payload = superjson.stringify(scene);
  const restored = superjson.parse<{ elements: unknown[] }>(payload);
  return { payload, restored };
};
