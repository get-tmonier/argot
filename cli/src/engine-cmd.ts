import { version } from './version.ts';

export function engineCmd(module: string): { cmd: string; args: string[] } {
  if (process.env['ARGOT_DEV'] === '1') {
    return {
      cmd: 'uv',
      args: ['run', '--package', 'argot-engine', 'python', '-m', module],
    };
  }
  return {
    cmd: 'uvx',
    args: [
      '--refresh-package',
      'argot-engine',
      '--from',
      `argot-engine==${version}`,
      'python',
      '-m',
      module,
    ],
  };
}
