// Break: a bare `throw new Error(...)` where Option's own convention is
// InvalidArgumentError — the repo's parseArg/choices callbacks always throw
// InvalidArgumentError so commander can report it via Command#error with the
// right exit code instead of an uncaught generic Error.
export function parsePort(value) {
  const port = Number(value);
  if (!Number.isInteger(port) || port <= 0) {
    throw new Error(`invalid port: ${value}`);
  }
  return port;
}
