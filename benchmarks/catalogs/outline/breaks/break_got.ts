import got from "got";

// Break: got HTTP client where outline networking goes through the shared ApiClient.
export async function fetchOpenGraph(url: string) {
  const response = await got(url, {
    method: "GET",
    timeout: { request: 5000 },
    retry: { limit: 2 },
    responseType: "json",
  });
  return response.body as { title: string; description: string };
}
