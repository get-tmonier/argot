import axios from "axios";

const SCENE_API = "https://scenes.example.com";

// Break: axios.request() HTTP client where excalidraw's collab data layer
// loads through fetch/firebase; the axios import sits in the decoy region
// above the hunk, so only the call-receiver stage sees the foreign callee.
// axios is 0-usage at the pinned SHA and absent from package.json.
export const pullRoomScene = async (roomId: string) => {
  const res = await axios.request({
    url: `${SCENE_API}/rooms/${roomId}`,
    method: "GET",
  });
  return res.data.elements as unknown[];
};
