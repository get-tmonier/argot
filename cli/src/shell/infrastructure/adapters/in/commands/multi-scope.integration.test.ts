import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { mkdtemp, rm, writeFile, mkdir, readFile } from 'node:fs/promises';
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
  workDir = await mkdtemp(join(tmpdir(), 'argot-multi-'));

  // Try --initial-branch first; fall back to init + checkout for older git
  const initResult = run('git', ['init', '--initial-branch=main'], workDir);
  if (initResult.code !== 0) {
    run('git', ['init'], workDir);
    run('git', ['checkout', '-b', 'main'], workDir);
  }
  await git(['config', 'user.email', 'test@example.com']);
  await git(['config', 'user.name', 'Test']);

  await mkdir(join(workDir, 'cli'), { recursive: true });
  await mkdir(join(workDir, 'engine'), { recursive: true });

  await writeFile(join(workDir, 'cli', 'foo.ts'), 'export const x = 1;\n');
  await writeFile(join(workDir, 'engine', 'bar.py'), 'x = 1\n');
  await git(['add', '.']);
  await git(['commit', '-m', 'init']);

  await writeFile(join(workDir, 'cli', 'foo.ts'), 'export const x = 1;\nexport const y = 2;\n');
  await writeFile(join(workDir, 'engine', 'bar.py'), 'x = 1\ny = 2\n');
  await git(['commit', '-am', 'add y']);

  await mkdir(join(workDir, '.argot'), { recursive: true });
  await writeFile(
    join(workDir, '.argot', 'config.json'),
    JSON.stringify({
      scopes: [
        { name: 'cli', path: 'cli/' },
        { name: 'engine', path: 'engine/' },
      ],
    }),
  );
});

afterAll(async () => {
  await rm(workDir, { recursive: true, force: true });
});

describe('multi-scope extract', () => {
  it('writes one dataset per scope, each containing only records under its prefix', async () => {
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

    const cliData = await readFile(join(workDir, '.argot/models/cli/dataset.jsonl'), 'utf-8');
    const engineData = await readFile(
      join(workDir, '.argot/models/engine/dataset.jsonl'),
      'utf-8',
    );

    const cliLines = cliData.trim().split('\n').filter(Boolean);
    const engineLines = engineData.trim().split('\n').filter(Boolean);

    expect(cliLines.length).toBeGreaterThan(0);
    expect(engineLines.length).toBeGreaterThan(0);

    for (const line of cliLines) {
      const record = JSON.parse(line);
      expect(record.file_path.startsWith('cli/')).toBe(true);
    }
    for (const line of engineLines) {
      const record = JSON.parse(line);
      expect(record.file_path.startsWith('engine/')).toBe(true);
    }
  }, 120_000);
});
