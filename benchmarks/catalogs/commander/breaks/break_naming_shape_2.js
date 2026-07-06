// Break: Hungarian-notation locals (bIsFatal, nExitCode, strCode) in an
// otherwise plain-camelCase file — CommanderError's own fields are exitCode/
// code/message with no type-prefix convention.
export function describeCommanderError(err) {
  const bIsFatal = err.exitCode !== 0;
  const nExitCode = err.exitCode;
  const strCode = err.code;
  return `${strCode} (exit ${nExitCode}, fatal=${bIsFatal})`;
}
