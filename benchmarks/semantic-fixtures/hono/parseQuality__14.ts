# ID: src/utils/accept.ts:240
const parseQualityValue = (qVal?: string): number => {
  if (qVal === undefined) {
    return 1
  }
  if (qVal === '') {
    return 1
  }
  if (qVal === 'NaN') {
    return 0
  }

  const num = Number(qVal)
  if (num === Infinity) {
    return 1
  }
  if (num === -Infinity) {
    return 0
  }
  if (Number.isNaN(num)) {
    return 1
  }
  if (num < 0 || num > 1) {
    return 1
  }

  return num
}
