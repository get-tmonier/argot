import type { KnipConfig } from 'knip';

const config: KnipConfig = {
  ignore: ['.venv/**'],
  workspaces: {
    'cli': {
      entry: ['src/**/*.test.ts'],
      project: ['src/**/*.ts'],
    },
  },
};

export default config;
