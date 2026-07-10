# ID: lib/help.js:696
function wrapAtWhitespace(helper, str, width) {
  if (width < helper.minWidthToWrap) return str;

  // split up text by whitespace
  const chunkPattern = /[\s]*[^\s]+/g;
  const rawLines = str.split(/\r\n|\n/);
  const wrappedLines = [];
  rawLines.forEach((line) => {
    const chunks = line.match(chunkPattern);
    if (chunks === null) {
      wrappedLines.push('');
      return;
    }

    let sumChunks = [chunks.shift()];
    let sumWidth = helper.displayWidth(sumChunks[0]);
    chunks.forEach((chunk) => {
      const visibleWidth = helper.displayWidth(chunk);
      // Accumulate chunks while they fit into width.
      if (sumWidth + visibleWidth <= width) {
        sumChunks.push(chunk);
        sumWidth += visibleWidth;
        return;
      }
      wrappedLines.push(sumChunks.join(''));
      const nextChunk = chunk.trimStart(); // trim space at line break
      sumChunks = [nextChunk];
      sumWidth = helper.displayWidth(nextChunk);
    });
    wrappedLines.push(sumChunks.join(''));
  });

  return wrappedLines.join('\n');
}
