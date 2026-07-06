import * as Comlink from "comlink";

// Break: Comlink worker RPC where outline offloads work through Bull queues.
export function exposeDocumentApi(api: {
  render: (doc: string) => string;
  outline: (doc: string) => string[];
}) {
  Comlink.expose(api);
  return Comlink.proxy(api);
}
