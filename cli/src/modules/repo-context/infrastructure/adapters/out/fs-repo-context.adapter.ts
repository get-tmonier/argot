import { spawn } from 'node:child_process';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { Effect, Layer } from 'effect';
import * as v from 'valibot';
import { RepoContext } from '#modules/repo-context/application/ports/out/repo-context.port.ts';
import { ScopeConfigInvalid, SettingsReadError } from '#modules/repo-context/domain/errors.ts';
import type { RepoStatus } from '#modules/repo-context/domain/repo-context.ts';
import { resolveScopes, ScopesFileSchema } from '#modules/repo-context/domain/scopes.ts';
import type { ResolvedScope } from '#modules/repo-context/domain/scopes.ts';
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

async function readScopesConfigAsync(gitRoot: string): Promise<ResolvedScope[]> {
  const configPath = join(gitRoot, '.argot', 'config.json');
  let raw: string;
  try {
    raw = await readFile(configPath, 'utf-8');
  } catch {
    return resolveScopes(gitRoot, undefined);
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (cause) {
    throw new ScopeConfigInvalid({ cause });
  }

  const result = v.safeParse(ScopesFileSchema, parsed);
  if (!result.success) {
    throw new ScopeConfigInvalid({ cause: result.issues });
  }

  try {
    return resolveScopes(gitRoot, result.output);
  } catch (cause) {
    throw new ScopeConfigInvalid({ cause });
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
          const scopes = await readScopesConfigAsync(gitRoot);

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
            datasetPath: scopes[0]!.datasetPath,
            modelPath: scopes[0]!.modelPath,
            scopes,
            preferences,
          };
        },
        catch: (e) => (e instanceof ScopeConfigInvalid ? e : new SettingsReadError({ cause: e })),
      }),

    listRepos: () =>
      Effect.tryPromise({
        try: async () => {
          const currentRoot = await getGitRootAsync();
          const global = await readGlobalSettingsAsync();

          const results: RepoStatus[] = [];
          for (const [path, entry] of Object.entries(global.repos)) {
            let scopesResolved: ResolvedScope[];
            try {
              scopesResolved = await readScopesConfigAsync(path);
            } catch {
              scopesResolved = resolveScopes(path, undefined);
            }

            const scopeStatuses = await Promise.all(
              scopesResolved.map(async (s) => ({
                name: s.name,
                pathPrefix: s.pathPrefix,
                datasetInfo: await statOrNull(s.datasetPath),
                modelInfo: await statOrNull(s.modelPath),
              })),
            );

            const primary = scopeStatuses[0]!;
            results.push({
              path,
              name: entry.name,
              isCurrent: path === currentRoot,
              datasetInfo: primary.datasetInfo,
              modelInfo: primary.modelInfo,
              scopes: scopeStatuses,
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
