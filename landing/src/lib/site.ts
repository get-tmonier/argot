/** Site-wide constants. */
export const SITE = {
  name: 'argot',
  domain: 'https://argot.tmonier.com',
  github: 'https://github.com/get-tmonier/argot',
  npm: 'https://www.npmjs.com/package/@tmonier/argot',
  install: 'curl -fsSL https://raw.githubusercontent.com/get-tmonier/argot/main/install.sh | sh',
  portfolio: 'https://tmonier.com',
  author: 'Damien Meur',
} as const;

/** Resolve a path for the current locale (en is unprefixed, fr is /fr-prefixed). */
export function localePath(path: string, locale: 'en' | 'fr'): string {
  const clean = path.startsWith('/') ? path : `/${path}`;
  if (locale === 'fr') {
    return clean === '/' ? '/fr/' : `/fr${clean}`;
  }
  return clean;
}
