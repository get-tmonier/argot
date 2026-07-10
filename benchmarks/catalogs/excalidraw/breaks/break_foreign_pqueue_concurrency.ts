import PQueue from "p-queue";

// Break: p-queue spins up a foreign promise-queue runtime to throttle
// outgoing broadcast messages where excalidraw's collab portal throttles
// with lodash.throttle and sends synchronously through the socket; p-queue
// is 0-usage at the pinned SHA and absent from package.json.
const broadcastQueue = new PQueue({ concurrency: 1 });

export const queueBroadcast = (send: () => Promise<void>) => {
  return broadcastQueue.add(send);
};
