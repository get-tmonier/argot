// Break: ejs template render (bare callee) where outline renders content through its own helpers.
export function renderExportManifest(
  template: string,
  data: { title: string; author: string; documents: number }
): string {
  return render(template, data, {
    rmWhitespace: true,
    cache: false,
  });
}
