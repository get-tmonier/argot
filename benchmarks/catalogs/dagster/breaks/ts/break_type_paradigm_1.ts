// Break: JSDoc-annotated JavaScript posing as TypeScript — heavy use of `any`, untyped function signatures,
// var declarations, and function() expressions. Dagit is strictly typed (no-any rule enforced, all
// component props/state/query results fully typed with explicit interfaces). This pattern predates
// the codebase's TypeScript adoption and would fail Dagit's strict tsc + oxlint/no-explicit-any checks.

/**
 * Build a schedule configuration object from raw form data.
 * @param {any} config
 * @returns {any}
 */
export function buildScheduleConfig(config: any): any {
  var result: any = {};
  result.name = config.name || 'unnamed';
  result.cronSchedule = config.cron;
  result.executionTimezone = config.tz || 'UTC';
  if (config.tags) {
    result.tags = config.tags.map(function (t: any) {
      return {key: t[0], value: t[1]};
    });
  } else {
    result.tags = [];
  }
  return result;
}

/**
 * Filter a list of runs by status string.
 * @param {any[]} runs
 * @param {string} status
 * @returns {any[]}
 */
export function filterRunsByStatus(runs: any[], status: string): any[] {
  return runs.filter(function (run: any) {
    return run.status === status;
  });
}

/**
 * Fire a callback when an asset materialization event is received.
 * @param {any} event
 * @param {Function} callback
 */
export function onAssetMaterialized(event: any, callback: Function): void {
  if (event && event.type === 'ASSET_MATERIALIZATION') {
    var payload: any = {
      assetKey: event.assetKey,
      runId: event.runId,
      timestamp: event.timestamp || Date.now(),
    };
    callback(payload);
  }
}
