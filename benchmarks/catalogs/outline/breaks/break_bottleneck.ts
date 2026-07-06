import Bottleneck from "bottleneck";

// Break: Bottleneck rate limiter where outline throttles via rate-limiter-flexible.
export function throttledGithubClient() {
  const limiter = new Bottleneck({
    maxConcurrent: 2,
    minTime: 250,
  });
  return (fetchFn: () => Promise<unknown>) =>
    limiter.schedule(() => fetchFn());
}
