import type { KnipConfig } from 'knip';

const config: KnipConfig = {
  workspaces: {
    'cli': {
      entry: ['src/**/*.test.ts'],
      project: ['src/**/*.ts'],
    },
  },
};

export default config;
