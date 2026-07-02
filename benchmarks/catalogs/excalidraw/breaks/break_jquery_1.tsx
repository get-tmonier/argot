import $ from "jquery";
import React from "react";

import type { Collaborator, SocketId } from "../types";

// Break: jQuery import at line 1 plus $-based DOM mutation inside a React
// component, in a codebase that is functional React with refs and CSS classes.
export const CollaboratorBadge = ({
  socketId,
  collaborator,
}: {
  socketId: SocketId;
  collaborator: Collaborator;
}) => {
  const toggleFollow = () => {
    $(`.collab-badge[data-socket="${socketId}"]`)
      .toggleClass("is-followed")
      .fadeIn(150);
    $("#user-list-panel").find(".collab-badge").not(`[data-socket="${socketId}"]`)
      .removeClass("is-followed");
  };

  return (
    <div
      className="collab-badge"
      data-socket={socketId}
      onClick={toggleFollow}
    >
      {collaborator.username ?? "Anonymous"}
    </div>
  );
};
