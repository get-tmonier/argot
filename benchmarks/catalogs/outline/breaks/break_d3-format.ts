// Break: d3-format number formatting (bare callee) where outline formats values via its own utils.
export function formatStorageBudget(bytes: number): string {
  const kb = format(",.1f")(bytes / 1024);
  const used = format(".0%")(bytes / (1024 * 1024));
  return `${kb} KB (${used} of budget)`;
}
