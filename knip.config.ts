import type { KnipConfig } from 'knip';

const config: KnipConfig = {
  ignore: ['.venv/**', 'npm/**'],
  workspaces: {
    'cli': {
      entry: ['src/**/*.test.ts'],
      project: ['src/**/*.ts'],
    },
  },
};

export default config;
