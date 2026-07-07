# ID: src/utils/url.ts:171
const expandOptionalParam = (routePath: string): string[] | null => {
  if (routePath.charCodeAt(routePath.length - 1) !== 63 || !routePath.includes(':')) {
    return null
  }

  const segments = routePath.split('/')
  const expanded: string[] = []
  let prefix = ''

  for (const segment of segments) {
    if (segment !== '' && !/\:/.test(segment)) {
      prefix += '/' + segment
    } else if (/\:/.test(segment)) {
      if (/\?/.test(segment)) {
        if (expanded.length === 0 && prefix === '') {
          expanded.push('/')
        } else {
          expanded.push(prefix)
        }
        const required = segment.replace('?', '')
        prefix += '/' + required
        expanded.push(prefix)
      } else {
        prefix += '/' + segment
      }
    }
  }

  return expanded.filter((v, i, a) => a.indexOf(v) === i)
}
