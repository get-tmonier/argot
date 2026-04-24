import fetch from 'node-fetch';

// Break: node-fetch import at line 1 inside a faker-js locale utility.
export async function resolveLocaleName(code: string): Promise<string> {
  const res = await fetch(`https://locale-api.example.com/${code}`);
  const json = (await res.json()) as { name: string };
  return json.name;
}
