import type { KnipConfig } from 'knip';

const config: KnipConfig = {
  ignore: ['.venv/**', 'npm/**', 'engine/argot/benchmark_fixtures/**'],
  workspaces: {
    'cli': {
      entry: ['src/**/*.test.ts'],
      project: ['src/**/*.ts'],
    },
  },
};

export default config;
