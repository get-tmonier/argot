# ID: src/modules/helpers/index.ts:218
function slugifyText(input: string = ''): string {
  return input
    .normalize('NFKD') // decompose accented chars into base + combining mark
    .replaceAll(/[̀-ͯ]/g, '') // strip the combining marks
    .replaceAll(' ', '-') // spaces become hyphens
    .replaceAll(/[^\w.-]+/g, ''); // drop everything but word chars, dots, hyphens
}
