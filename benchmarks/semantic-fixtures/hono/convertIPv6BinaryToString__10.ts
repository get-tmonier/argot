# ID: src/utils/ipaddr.ts:115
const stringifyIPv6Binary = (ipV6: bigint): string => {
  if (isIPv4MappedIPv6(ipV6)) {
    return `::ffff:${convertIPv4BinaryToString(convertIPv4MappedIPv6ToIPv4(ipV6))}`
  }

  const sections = []
  for (let i = 0; i < 8; i++) {
    sections.push(((ipV6 >> BigInt(16 * (7 - i))) & 0xffffn).toString(16))
  }

  // find the longest run of zero groups to collapse with '::'
  let runStart = -1
  let bestStart = -1
  let bestEnd = -1
  for (let i = 0; i < 8; i++) {
    if (sections[i] === '0') {
      if (runStart === -1) {
        runStart = i
      }
    } else {
      if (runStart > -1) {
        if (i - runStart > bestEnd - bestStart) {
          bestStart = runStart
          bestEnd = i
        }
        runStart = -1
      }
    }
  }
  if (runStart > -1) {
    if (8 - runStart > bestEnd - bestStart) {
      bestStart = runStart
      bestEnd = 8
    }
  }
  if (bestStart !== -1) {
    sections.splice(bestStart, bestEnd - bestStart, ':')
  }

  return sections.join(':').replace(/:{2,}/g, '::')
}
