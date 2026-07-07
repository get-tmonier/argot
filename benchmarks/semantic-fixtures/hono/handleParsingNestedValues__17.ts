# ID: src/utils/body.ts:206
const assignNestedValue = (
  form: BodyData,
  key: string,
  value: BodyDataValue<Partial<ParseBodyOptions>>
): void => {
  if (/(?:^|\.)__proto__\./.test(key)) {
    return
  }

  let cursor = form
  const parts = key.split('.')

  parts.forEach((part, index) => {
    if (index === parts.length - 1) {
      cursor[part] = value
    } else {
      if (
        !cursor[part] ||
        typeof cursor[part] !== 'object' ||
        Array.isArray(cursor[part]) ||
        cursor[part] instanceof File
      ) {
        cursor[part] = Object.create(null)
      }
      cursor = cursor[part] as unknown as BodyData
    }
  })
}
