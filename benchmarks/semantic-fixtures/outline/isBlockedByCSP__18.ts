# ID: server/utils/embeds.ts:53
function cspBlocksEmbedding(value: string | null): boolean {
  if (!value) {
    return false;
  }

  // Look for a frame-ancestors directive within the CSP header
  const directives = value.split(";").map((d) => d.trim());

  for (const directive of directives) {
    const tokens = directive.split(/\s+/);
    if (tokens[0]?.toLowerCase() !== "frame-ancestors") {
      continue;
    }

    const sources = tokens.slice(1);

    // 'none' cannot be embedded anywhere
    if (sources.length === 1 && sources[0] === "'none'") {
      return true;
    }

    // 'self' only means same-origin, which excludes us
    if (sources.length === 1 && sources[0] === "'self'") {
      return true;
    }

    // A wildcard anywhere means embedding is permitted
    if (sources.includes("*")) {
      return false;
    }

    // Specific origins without a wildcard: assume we are not listed
    if (
      sources.length > 0 &&
      !sources.every((s) => s === "'self'" || s === "'none'")
    ) {
      return true;
    }
  }

  return false;
}
