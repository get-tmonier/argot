# ID: src/utils/url.ts:51
const compilePattern = (segment: string, following?: string): Pattern | null => {
  if (segment === '*') {
    return '*'
  }

  const parsed = segment.match(/^\:([^\{\}]+)(?:\{(.+)\})?$/)
  if (!parsed) {
    return null
  }

  const key = `${segment}#${following}`
  if (patternCache[key] === undefined) {
    if (parsed[2]) {
      const hasPlainNext = following && following[0] !== ':' && following[0] !== '*'
      patternCache[key] = hasPlainNext
        ? [key, parsed[1], new RegExp(`^${parsed[2]}(?=/${following})`)]
        : [segment, parsed[1], new RegExp(`^${parsed[2]}$`)]
    } else {
      patternCache[key] = [segment, parsed[1], true]
    }
  }

  return patternCache[key]
}
