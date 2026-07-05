// Break: pusher-js realtime runtime lazy-loaded via a dynamic import() and
// reached through .subscribe()/.bind(), where excalidraw collaboration runs on
// socket.io-client. The leaf methods are attested repo vocabulary and the
// dynamic import() evades the import stage (only static import/require are
// modelled), so the foreign realtime runtime is masked from the import and
// call-receiver stages — only bpe token-surprise can catch it. pusher-js is
// 0-usage at the pinned SHA and absent from package.json.
export const watchRoomPresence = async (
  roomId: string,
  onUpdate: (socketId: string) => void,
) => {
  const { default: Pusher } = await import("pusher-js");
  const pusher = new Pusher("app-key", { cluster: "eu" });
  const channel = pusher.subscribe(`presence-${roomId}`);
  channel.bind("member_added", (member: { id: string }) => {
    onUpdate(member.id);
  });
  return channel;
};
