/** @type {import('dependency-cruiser').IConfiguration} */
module.exports = {
  forbidden: [
    {
      name: 'no-domain-imports-infra',
      comment: 'Domain must not depend on infrastructure or shell',
      severity: 'error',
      from: { path: 'cli/src/modules/[^/]+/domain/' },
      to: {
        path: [
          'cli/src/modules/[^/]+/infrastructure/',
          'cli/src/shell/',
        ],
      },
    },
    {
      name: 'no-application-imports-infra',
      comment: 'Application layer must not depend on infrastructure',
      severity: 'error',
      from: { path: 'cli/src/modules/[^/]+/application/' },
      to: { path: 'cli/src/modules/[^/]+/infrastructure/' },
    },
    {
      name: 'no-cross-module-deep',
      comment: 'Modules may not import deep into other modules',
      severity: 'error',
      from: { path: 'cli/src/modules/([^/]+)/' },
      to: {
        path: 'cli/src/modules/(?!\\1)([^/]+)/(domain|infrastructure|application)',
      },
    },
    {
      name: 'no-shell-imports-modules-infra',
      comment: 'Shell must not import module infrastructure directly',
      severity: 'error',
      from: { path: 'cli/src/shell/' },
      to: { path: 'cli/src/modules/[^/]+/infrastructure/' },
    },
  ],
  options: {
    doNotFollow: { path: 'node_modules' },
    moduleSystems: ['es6'],
    tsPreCompilationDeps: true,
    tsConfig: { fileName: 'cli/tsconfig.json' },
  },
};
