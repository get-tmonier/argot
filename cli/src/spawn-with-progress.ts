const UV_LINE_PATTERN = /^(Resolved|Prepared|Installed|Downloading|Audited|Updated)\b/;
const SPINNER_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

function createUvSpinner(message: string): { stop: () => void } {
  let frame = 0;
  const interval = setInterval(() => {
    process.stderr.write(`\r${SPINNER_FRAMES[frame++ % SPINNER_FRAMES.length]} ${message}  `);
  }, 80);
  return {
    stop() {
      clearInterval(interval);
      process.stderr.write('\r\x1b[K');
    },
  };
}

export function handleUvStderr(
  stderrStream: NodeJS.ReadableStream,
  onErrorChunk: (chunk: Buffer) => void,
): () => void {
  let spinner: ReturnType<typeof createUvSpinner> | null = null;

  stderrStream.on('data', (chunk: Buffer) => {
    const text = chunk.toString('utf-8');
    const nonUvLines: string[] = [];

    for (const line of text.split('\n')) {
      if (UV_LINE_PATTERN.test(line.trim())) {
        if (!spinner) spinner = createUvSpinner('Installing argot-engine…');
      } else if (line.trim()) {
        nonUvLines.push(line);
      }
    }

    if (nonUvLines.length > 0) {
      onErrorChunk(Buffer.from(nonUvLines.join('\n')));
    }
  });

  return () => {
    spinner?.stop();
    spinner = null;
  };
}
