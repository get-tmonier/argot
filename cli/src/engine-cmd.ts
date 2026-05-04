import { version } from './version.ts';

export function engineCmd(module: string): { cmd: string; args: string[] } {
  if (process.env['ARGOT_DEV'] === '1') {
    const project = process.env['ARGOT_DEV_PROJECT'];
    const projectArgs = project ? ['--project', project] : [];
    return {
      cmd: 'uv',
      args: ['run', ...projectArgs, '--package', 'argot-engine', 'python', '-m', module],
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
