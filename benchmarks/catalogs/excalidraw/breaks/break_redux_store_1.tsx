import { useSelector, useDispatch } from "react-redux";
import React from "react";

import type { LibraryItem } from "../types";

// Break: react-redux useSelector/useDispatch at line 1 in a codebase whose
// state lives in jotai atoms and the App class — redux is entirely absent.
export const LibraryPublishStatus = ({ item }: { item: LibraryItem }) => {
  const dispatch = useDispatch();
  const publishedIds = useSelector(
    (state: { library: { publishedIds: string[] } }) =>
      state.library.publishedIds,
  );

  const isPublished = publishedIds.includes(item.id);

  return (
    <button
      className="library-item__publish"
      onClick={() =>
        dispatch({
          type: isPublished ? "library/unpublish" : "library/publish",
          payload: item.id,
        })
      }
    >
      {isPublished ? "Unpublish" : "Publish"}
    </button>
  );
};
