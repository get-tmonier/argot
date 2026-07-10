# ID: lib/suggestSimilar.js:56
function closestCandidates(word, candidates) {
  if (!candidates || candidates.length === 0) return '';
  // drop any duplicates
  candidates = Array.from(new Set(candidates));

  const searchingOptions = word.startsWith('--');
  if (searchingOptions) {
    word = word.slice(2);
    candidates = candidates.map((candidate) => candidate.slice(2));
  }

  const minSimilarity = 0.4;
  let bestDistance = 3;
  let similar = [];
  for (const candidate of candidates) {
    if (candidate.length <= 1) continue; // no single-character guesses
    const distance = editDistance(word, candidate);
    const length = Math.max(word.length, candidate.length);
    const similarity = (length - distance) / length;
    if (similarity <= minSimilarity) continue;
    if (distance < bestDistance) {
      bestDistance = distance;
      similar = [candidate];
    } else if (distance === bestDistance) {
      similar.push(candidate);
    }
  }

  similar.sort((a, b) => a.localeCompare(b));
  if (searchingOptions) {
    similar = similar.map((candidate) => `--${candidate}`);
  }

  if (similar.length > 1) return `\n(Did you mean one of ${similar.join(', ')}?)`;
  if (similar.length === 1) return `\n(Did you mean ${similar[0]}?)`;
  return '';
}
