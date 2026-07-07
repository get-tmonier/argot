# ID: src/utils/buffer.ts:8
const buffersEqual = (a: ArrayBuffer, b: ArrayBuffer): boolean => {
  if (a === b) {
    return true
  }
  if (a.byteLength !== b.byteLength) {
    return false
  }

  const viewA = new DataView(a)
  const viewB = new DataView(b)

  let i = viewA.byteLength
  while (i--) {
    if (viewA.getUint8(i) !== viewB.getUint8(i)) {
      return false
    }
  }

  return true
}
