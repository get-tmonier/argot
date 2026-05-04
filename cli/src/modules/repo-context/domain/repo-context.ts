export interface ResolvedContext {
  gitRoot: string;
  name: string;
  argotDir: string;
  datasetPath: string;
  repoCorpusPath: string;
  genericBaselinePath: string;
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
