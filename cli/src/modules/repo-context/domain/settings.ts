export interface Preferences {
  threshold: number;
  model: string;
}

export const DEFAULT_PREFERENCES: Preferences = {
  threshold: 0.5,
  model: 'sonnet',
};

export interface RepoEntry {
  name: string;
  registeredAt: string;
  lastUsedAt: string;
}

export interface GlobalSettings {
  version: number;
  preferences: Preferences;
  repos: Record<string, RepoEntry>;
}

export const DEFAULT_GLOBAL_SETTINGS: GlobalSettings = {
  version: 1,
  preferences: DEFAULT_PREFERENCES,
  repos: {},
};

export interface LocalSettings {
  preferences?: Partial<Preferences>;
}

export function mergePreferences(
  global: Preferences,
  local: Partial<Preferences> | undefined,
): Preferences {
  if (!local) return global;
  return { ...global, ...local };
}
