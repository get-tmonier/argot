export interface RepoEntry {
  name: string;
  registeredAt: string;
  lastUsedAt: string;
}

export interface GlobalSettings {
  version: number;
  repos: Record<string, RepoEntry>;
}

export const DEFAULT_GLOBAL_SETTINGS: GlobalSettings = {
  version: 1,
  repos: {},
};
