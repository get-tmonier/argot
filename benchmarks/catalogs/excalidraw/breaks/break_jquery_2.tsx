import React, { useEffect } from "react";
import $ from "jquery";

import type { SearchMatchItem } from "../types";

export const SearchMatchHighlighter = ({
  matches,
}: {
  matches: readonly SearchMatchItem[];
}) => {
  useEffect(() => {
    // Break: jQuery event delegation and animation where the codebase uses
    // React refs, controlled state, and CSS transitions.
    $(".layer-ui__search").on("click", ".search-match", function () {
      $(this).addClass("active").siblings().removeClass("active");
      $(this).find(".preview").slideDown(200);
    });
    $(".search-match .preview").hide();

    return () => {
      $(".layer-ui__search").off("click", ".search-match");
    };
  }, [matches]);

  return (
    <div className="layer-ui__search">
      {matches.map((match, index) => (
        <div key={index} className="search-match">
          <span className="preview">{match.searchQuery}</span>
        </div>
      ))}
    </div>
  );
};
