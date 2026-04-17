import type { KnipConfig } from 'knip';

const config: KnipConfig = {
  workspaces: {
    'cli': {
      project: ['src/**/*.ts'],
    },
  },
};

export default config;
