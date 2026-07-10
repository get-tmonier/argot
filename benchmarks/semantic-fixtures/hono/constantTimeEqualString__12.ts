# ID: src/utils/buffer.ts:29
const constantTimeStringCompare = (a: string, b: string): boolean => {
  const lenA = a.length
  const lenB = b.length
  const longest = Math.max(lenA, lenB)
  let diff = lenA ^ lenB
  for (let i = 0; i < longest; i++) {
    const charA = i < lenA ? a.charCodeAt(i) : 0
    const charB = i < lenB ? b.charCodeAt(i) : 0
    diff |= charA ^ charB
  }
  return diff === 0
}
