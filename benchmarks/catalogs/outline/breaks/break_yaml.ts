// Break: yaml.parse frontmatter (bare callee) where outline parses YAML with js-yaml.
export function extractFrontmatter(raw: string): Record<string, unknown> {
  const end = raw.indexOf("\n---\n");
  const header = end === -1 ? "" : raw.slice(4, end);
  const meta = parse(header) as Record<string, unknown>;
  return { ...meta, hasBody: end !== -1 };
}
