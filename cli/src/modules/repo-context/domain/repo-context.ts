import type { Preferences } from './settings.ts';

export interface ResolvedContext {
  gitRoot: string;
  name: string;
  argotDir: string;
  datasetPath: string;
  modelAPath: string;
  modelBPath: string;
  preferences: Preferences;
}

export interface DatasetInfo {
  sizeBytes: number;
  mtime: Date;
}

export interface ModelInfo {
  sizeBytes: number;
  mtime: Date;
}

export interface RepoStatus {
  path: string;
  name: string;
  isCurrent: boolean;
  datasetInfo: DatasetInfo | null;
  modelInfo: ModelInfo | null;
}
