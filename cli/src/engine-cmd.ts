/**
 * Resolves the engine command prefix based on the runtime environment.
 * In dev (ARGOT_DEV=1), delegates to the local uv workspace.
 * In production (compiled binary), downloads the published package via uvx.
 */
export function engineCmd(module: string): { cmd: string; args: string[] } {
  if (process.env['ARGOT_DEV'] === '1') {
    return {
      cmd: 'uv',
      args: ['run', '--package', 'argot-engine', 'python', '-m', module],
    };
  }
  return {
    cmd: 'uvx',
    args: ['--from', 'argot-engine', 'python', '-m', module],
  };
}
