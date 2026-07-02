type SearchResult = {
  documentId: string;
  title: string;
  context: string;
};

// Break: hand-rolled XMLHttpRequest state machine with callbacks where the voice is a promise-based ApiClient.
export function searchDocuments(
  query: string,
  onSuccess: (results: SearchResult[]) => void,
  onError: (status: number) => void
) {
  const xhr = new XMLHttpRequest();
  xhr.open("POST", "/api/documents.search", true);
  xhr.setRequestHeader("Content-Type", "application/json");

  xhr.onreadystatechange = function () {
    if (xhr.readyState !== XMLHttpRequest.DONE) {
      return;
    }
    if (xhr.status >= 200 && xhr.status < 300) {
      const body = JSON.parse(xhr.responseText);
      onSuccess(body.data as SearchResult[]);
    } else {
      onError(xhr.status);
    }
  };

  xhr.ontimeout = function () {
    onError(0);
  };

  xhr.timeout = 10000;
  xhr.send(JSON.stringify({ query }));
}
