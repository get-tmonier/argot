import type { KnipConfig } from 'knip';

const config: KnipConfig = {
  ignore: ['.venv/**', 'npm/**', 'engine/argot/acceptance/**', 'benchmarks/**'],
  workspaces: {
    'cli': {
      entry: ['src/**/*.test.ts'],
      project: ['src/**/*.ts'],
    },
  },
};

export default config;
