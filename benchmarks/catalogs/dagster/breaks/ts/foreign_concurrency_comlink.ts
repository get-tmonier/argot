// Break: Comlink web-worker RPC wrapping instead of Dagit's @koale/useworker / worker-loader pattern.
// Dagit offloads heavy graph computation with worker-loader workers and @koale/useworker hooks. Comlink's
// wrap()/proxy() RPC bridge is a different web-worker concurrency library that ui-core does not import.
import * as Comlink from 'comlink';

interface GraphComputeApi {
  computeLayout(nodes: unknown[], edges: unknown[]): Promise<{width: number; height: number}>;
}

export function connectGraphWorker(worker: Worker): Comlink.Remote<GraphComputeApi> {
  return Comlink.wrap<GraphComputeApi>(worker);
}

export async function computeLayoutOffThread(
  worker: Worker,
  nodes: unknown[],
  edges: unknown[],
): Promise<{width: number; height: number}> {
  const api = Comlink.wrap<GraphComputeApi>(worker);
  return api.computeLayout(nodes, edges);
}
