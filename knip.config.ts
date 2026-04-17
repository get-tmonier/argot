import type { KnipConfig } from 'knip';

const config: KnipConfig = {
  entry: ['cli/src/cli.ts'],
  project: ['cli/src/**/*.ts'],
  ignore: ['engine/**'],
};

export default config;
