import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { access, constants, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

let workDir: string;

function run(
  cmd: string,
  args: string[],
  cwd: string,
  uvProject?: string,
): { code: number; stderr: string; stdout: string } {
  const result = spawnSync(cmd, args, {
    cwd,
    encoding: 'utf-8',
    env: {
      ...process.env,
      ARGOT_DEV: '1',
      ...(uvProject ? { UV_PROJECT: uvProject } : {}),
    },
  });
  return {
    code: result.status ?? -1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

async function git(args: string[]): Promise<void> {
  const r = run('git', args, workDir);
  if (r.code !== 0) throw new Error(`git ${args.join(' ')} → ${r.code}: ${r.stderr}`);
}

beforeAll(async () => {
  workDir = await mkdtemp(join(tmpdir(), 'argot-single-'));

  // Try --initial-branch first; fall back to init + checkout for older git
  const initResult = run('git', ['init', '--initial-branch=main'], workDir);
  if (initResult.code !== 0) {
    run('git', ['init'], workDir);
    run('git', ['checkout', '-b', 'main'], workDir);
  }
  await git(['config', 'user.email', 'test@example.com']);
  await git(['config', 'user.name', 'Test']);

  await mkdir(join(workDir, 'src'), { recursive: true });
  await writeFile(join(workDir, 'src', 'foo.ts'), 'export const x = 1;\n');
  await git(['add', '.']);
  await git(['commit', '-m', 'init']);

  await writeFile(join(workDir, 'src', 'foo.ts'), 'export const x = 1;\nexport const y = 2;\n');
  await git(['commit', '-am', 'update']);
});

afterAll(async () => {
  await rm(workDir, { recursive: true, force: true });
});

describe('no config (single default scope)', () => {
  it('writes dataset to .argot/dataset.jsonl — no models/ subdirectory', async () => {
    // import.meta.dir = .../cli/src/shell/infrastructure/adapters/in/commands
    // 5 levels up → .../cli/src
    const cliEntry = join(import.meta.dir, '..', '..', '..', '..', '..', 'cli.ts');
    // 7 levels up → argot repo root (has workspace pyproject.toml for uv)
    const argotRoot = join(import.meta.dir, '..', '..', '..', '..', '..', '..', '..');
    const r = run('bun', ['run', cliEntry, 'extract'], workDir, argotRoot);

    if (r.code !== 0) {
      console.error('stdout:', r.stdout);
      console.error('stderr:', r.stderr);
    }
    expect(r.code).toBe(0);

    await access(join(workDir, '.argot', 'dataset.jsonl'), constants.F_OK);
    await expect(access(join(workDir, '.argot', 'models'), constants.F_OK)).rejects.toThrow();
  }, 120_000);
});
