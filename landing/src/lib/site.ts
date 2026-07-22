/** Site-wide constants. */
export const SITE = {
  name: 'argot',
  domain: 'https://argot.tmonier.com',
  github: 'https://github.com/get-tmonier/argot',
  npm: 'https://www.npmjs.com/package/@tmonier/argot',
  install:
    "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh",
  portfolio: 'https://tmonier.com',
  author: 'Damien Meur',
} as const;

/** Resolve a path for the current locale (en is unprefixed, fr is /fr-prefixed). */
export function localePath(path: string, locale: 'en' | 'fr'): string {
  const clean = path.startsWith('/') ? path : `/${path}`;
  // French is currently published only for the landing and proof-status routes.
  // Keep shared docs and legal routes unprefixed until localized routes exist.
  if (locale === 'fr' && (clean === '/' || clean === '/caught-in-the-wild/'))
    return clean === '/' ? '/fr/' : `/fr${clean}`;
  return clean;
}

/** Whether this route has an equivalent page in the other published locale. */
export function hasLocaleAlternate(pathname: string): boolean {
  return (
    pathname === '/' ||
    pathname === '/fr/' ||
    pathname === '/caught-in-the-wild/' ||
    pathname === '/fr/caught-in-the-wild/'
  );
}
