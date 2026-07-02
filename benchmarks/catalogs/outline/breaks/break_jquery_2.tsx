import $ from "jquery";
import * as React from "react";

// Break: jQuery import at line 1 plus $.ajax where the voice is the shared ApiClient util.
function RecentDocuments() {
  const [titles, setTitles] = React.useState<string[]>([]);

  React.useEffect(() => {
    $.ajax({
      url: "/api/documents.list",
      method: "POST",
      dataType: "json",
      success: (res: { data: Array<{ title: string }> }) => {
        setTitles(res.data.map((doc) => doc.title));
        $("#recent-docs").fadeIn();
      },
      error: () => {
        $("#recent-docs").hide();
      },
    });
  }, []);

  return (
    <ul id="recent-docs">
      {titles.map((title) => (
        <li key={title}>{title}</li>
      ))}
    </ul>
  );
}

export default RecentDocuments;
