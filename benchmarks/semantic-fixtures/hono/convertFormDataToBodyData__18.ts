# ID: src/utils/body.ts:142
function formDataToBodyData<T extends BodyData = BodyData>(
  formData: FormData,
  options: ParseBodyOptions
): T {
  const form: BodyData = Object.create(null)

  formData.forEach((value, key) => {
    const collectAll = options.all || key.endsWith('[]')

    if (!collectAll) {
      form[key] = value
    } else {
      handleParsingAllValues(form, key, value)
    }
  })

  if (options.dot) {
    Object.entries(form).forEach(([key, value]) => {
      const isDotted = key.includes('.')

      if (isDotted) {
        handleParsingNestedValues(form, key, value)
        delete form[key]
      }
    })
  }

  return form as T
}
