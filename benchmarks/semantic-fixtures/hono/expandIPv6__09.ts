# ID: src/utils/ipaddr.ts:13
const normalizeIPv6 = (ipV6: string): string => {
  const sections = ipV6.split(':')
  if (IPV4_REGEX.test(sections.at(-1) as string)) {
    sections.splice(
      -1,
      1,
      ...convertIPv6BinaryToString(convertIPv4ToBinary(sections.at(-1) as string)) // => ::7f00:0001
        .substring(2) // => 7f00:0001
        .split(':') // => ['7f00', '0001']
    )
  }
  for (let i = 0; i < sections.length; i++) {
    const node = sections[i]
    if (node !== '') {
      sections[i] = node.padStart(4, '0')
    } else {
      sections[i + 1] === '' && sections.splice(i + 1, 1)
      sections[i] = new Array(8 - sections.length + 1).fill('0000').join(':')
    }
  }
  return sections.join(':')
}
