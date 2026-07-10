# ID: lib/suggestSimilar.js:3
const editCeiling = 3;

function computeEditDistance(source, target) {
  // Optimal string alignment (Damerau-Levenshtein), no substring edited twice.
  if (Math.abs(source.length - target.length) > editCeiling) {
    return Math.max(source.length, target.length);
  }

  const matrix = [];
  for (let row = 0; row <= source.length; row++) {
    matrix[row] = [row];
  }
  for (let col = 0; col <= target.length; col++) {
    matrix[0][col] = col;
  }

  for (let col = 1; col <= target.length; col++) {
    for (let row = 1; row <= source.length; row++) {
      const substitutionCost = source[row - 1] === target[col - 1] ? 0 : 1;
      matrix[row][col] = Math.min(
        matrix[row - 1][col] + 1, // deletion
        matrix[row][col - 1] + 1, // insertion
        matrix[row - 1][col - 1] + substitutionCost, // substitution
      );
      const transposed =
        row > 1 &&
        col > 1 &&
        source[row - 1] === target[col - 2] &&
        source[row - 2] === target[col - 1];
      if (transposed) {
        matrix[row][col] = Math.min(matrix[row][col], matrix[row - 2][col - 2] + 1);
      }
    }
  }

  return matrix[source.length][target.length];
}
