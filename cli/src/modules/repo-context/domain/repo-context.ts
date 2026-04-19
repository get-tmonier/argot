import type { Preferences } from './settings.ts';
import type { ResolvedScope } from './scopes.ts';

export interface ResolvedContext {
  gitRoot: string;
  name: string;
  datasetPath: string;
  modelPath: string;
  scopes: ResolvedScope[];
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

export interface ScopeStatus {
  name: string;
  pathPrefix: string;
  datasetInfo: DatasetInfo | null;
  modelInfo: ModelInfo | null;
}

export interface RepoStatus {
  path: string;
  name: string;
  isCurrent: boolean;
  datasetInfo: DatasetInfo | null;
  modelInfo: ModelInfo | null;
  scopes: ScopeStatus[];
}
