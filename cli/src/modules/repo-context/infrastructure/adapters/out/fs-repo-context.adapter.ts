import { spawn } from 'node:child_process';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { Effect, Layer } from 'effect';
import { RepoContext } from '#modules/repo-context/application/ports/out/repo-context.port.ts';
import { SettingsReadError } from '#modules/repo-context/domain/errors.ts';
import type {
  DatasetInfo,
  ModelInfo,
  RepoStatus,
} from '#modules/repo-context/domain/repo-context.ts';
import {
  DEFAULT_GLOBAL_SETTINGS,
  mergePreferences,
  type GlobalSettings,
  type LocalSettings,
} from '#modules/repo-context/domain/settings.ts';

const GLOBAL_SETTINGS_PATH = join(homedir(), '.argot', 'settings.json');

async function getGitRootAsync(): Promise<string | null> {
  return new Promise((resolve) => {
    const proc = spawn('git', ['rev-parse', '--show-toplevel'], {
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    const chunks: Buffer[] = [];
    proc.stdout!.on('data', (chunk: Buffer) => chunks.push(chunk));
    proc.on('close', (code) => {
      resolve(code === 0 ? Buffer.concat(chunks).toString('utf-8').trim() : null);
    });
    proc.on('error', () => resolve(null));
  });
}

async function readGlobalSettingsAsync(): Promise<GlobalSettings> {
  try {
    const content = await readFile(GLOBAL_SETTINGS_PATH, 'utf-8');
    return JSON.parse(content) as GlobalSettings;
  } catch {
    return structuredClone(DEFAULT_GLOBAL_SETTINGS);
  }
}

async function writeGlobalSettingsAsync(settings: GlobalSettings): Promise<void> {
  await mkdir(dirname(GLOBAL_SETTINGS_PATH), { recursive: true });
  await writeFile(GLOBAL_SETTINGS_PATH, JSON.stringify(settings, null, 2));
}

async function readLocalSettingsAsync(gitRoot: string): Promise<LocalSettings | null> {
  try {
    const content = await readFile(join(gitRoot, '.argot', 'settings.json'), 'utf-8');
    return JSON.parse(content) as LocalSettings;
  } catch {
    return null;
  }
}

async function statOrNull(path: string): Promise<{ sizeBytes: number; mtime: Date } | null> {
  try {
    const s = await stat(path);
    return { sizeBytes: s.size, mtime: s.mtime };
  } catch {
    return null;
  }
}

export const FsRepoContextLive = Layer.effect(RepoContext)(
  Effect.succeed({
    resolveContext: () =>
      Effect.tryPromise({
        try: async () => {
          const gitRoot = (await getGitRootAsync()) ?? process.cwd();
          const global = await readGlobalSettingsAsync();
          const local = await readLocalSettingsAsync(gitRoot);
          const preferences = mergePreferences(global.preferences, local?.preferences);

          const now = new Date().toISOString();
          if (!global.repos[gitRoot]) {
            global.repos[gitRoot] = {
              name: basename(gitRoot),
              registeredAt: now,
              lastUsedAt: now,
            };
          } else {
            global.repos[gitRoot]!.lastUsedAt = now;
          }
          await writeGlobalSettingsAsync(global);

          return {
            gitRoot,
            name: global.repos[gitRoot]!.name,
            datasetPath: join(gitRoot, '.argot', 'dataset.jsonl'),
            modelPath: join(gitRoot, '.argot', 'model.pkl'),
            preferences,
          };
        },
        catch: (e) => new SettingsReadError({ cause: e }),
      }),

    listRepos: () =>
      Effect.tryPromise({
        try: async () => {
          const currentRoot = await getGitRootAsync();
          const global = await readGlobalSettingsAsync();

          const results: RepoStatus[] = [];
          for (const [path, entry] of Object.entries(global.repos)) {
            const datasetInfo: DatasetInfo | null = await statOrNull(
              join(path, '.argot', 'dataset.jsonl'),
            );
            const modelInfo: ModelInfo | null = await statOrNull(join(path, '.argot', 'model.pkl'));
            results.push({
              path,
              name: entry.name,
              isCurrent: path === currentRoot,
              datasetInfo,
              modelInfo,
            });
          }
          return results.sort((a, b) => a.name.localeCompare(b.name));
        },
        catch: (e) => new SettingsReadError({ cause: e }),
      }),

    readGlobalSettings: () =>
      Effect.tryPromise({
        try: () => readGlobalSettingsAsync(),
        catch: (e) => new SettingsReadError({ cause: e }),
      }),
  }),
);
