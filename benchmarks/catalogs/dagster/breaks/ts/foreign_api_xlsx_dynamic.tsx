// Break: dynamic import('xlsx') SheetJS export — foreign dep loaded via a dynamic import() expression.
// Dagit has no spreadsheet-export path; run tables render through @dagster-io/ui-components. This reaches the
// foreign `xlsx` (SheetJS) package through a dynamic import() EXPRESSION, deliberately not a static import
// statement, so import-graph analysis (which only sees `import ... from`) never observes the dependency.
// xlsx is 0-usage in ui-core.
export async function exportRunsToXlsx(
  rows: Array<Record<string, unknown>>,
  fileName: string,
): Promise<void> {
  const XLSX = await import('xlsx');
  const worksheet = XLSX.utils.json_to_sheet(rows);
  const workbook = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(workbook, worksheet, 'Runs');
  XLSX.writeFile(workbook, fileName);
}
