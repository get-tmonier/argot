# ID: lib/option.js:328
function separateFlagForms(flags) {
  let shortFlag;
  let longFlag;
  const shortFlagExp = /^-[^-]$/;
  const longFlagExp = /^--[^-]/;

  const flagParts = flags.split(/[ |,]+/).concat('guard');
  // Normal ordering is short and/or long.
  if (shortFlagExp.test(flagParts[0])) shortFlag = flagParts.shift();
  if (longFlagExp.test(flagParts[0])) longFlag = flagParts.shift();
  // Long then short, rarely used but fine.
  if (!shortFlag && shortFlagExp.test(flagParts[0])) {
    shortFlag = flagParts.shift();
  }
  // Two long flags, like '--ws, --workspace'.
  if (!shortFlag && longFlagExp.test(flagParts[0])) {
    shortFlag = longFlag;
    longFlag = flagParts.shift();
  }

  // Fail noisily on an unprocessed flag rather than silently ignore it.
  if (flagParts[0].startsWith('-')) {
    const unsupportedFlag = flagParts[0];
    throw new Error(
      `option creation failed due to '${unsupportedFlag}' in option flags '${flags}'`,
    );
  }
  if (shortFlag === undefined && longFlag === undefined) {
    throw new Error(`option creation failed due to no flags found in '${flags}'.`);
  }

  return { shortFlag, longFlag };
}
