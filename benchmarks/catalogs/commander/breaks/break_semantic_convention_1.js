// Break: raw process.exit bypassing Command's own error convention — every
// exit path in commander is routed through Command#error/_exit so
// _exitCallback and CommanderError still fire; calling process.exit directly
// from an action handler skips that machinery entirely.
export function forceQuitOnVersionMismatch(current, required) {
  if (current !== required) {
    console.error(`version mismatch: need ${required}, found ${current}`);
    process.exit(1);
  }
}
