import moment from "moment";

import type { Collaborator, SocketId } from "./types";

export const formatCollaboratorPresence = (
  collaborators: Map<SocketId, Collaborator>,
) => {
  const lines: string[] = [];
  for (const [socketId, collaborator] of collaborators) {
    // Break: moment.js chained date arithmetic and humanized output for
    // presence labels, where the codebase stores/compares epoch millis
    // directly (Date.now()) and never uses a date library.
    const lastActive = moment(collaborator.lastActiveTime ?? Date.now());
    const label = lastActive.isAfter(moment().subtract(30, "seconds"))
      ? "active now"
      : lastActive.clone().local().fromNow();
    lines.push(`${collaborator.username ?? socketId}: ${label}`);
  }
  return lines;
};

export const isIdleSince = (lastActiveTime: number | undefined) => {
  return moment(lastActiveTime ?? 0)
    .add(3, "minutes")
    .isBefore(moment());
};
