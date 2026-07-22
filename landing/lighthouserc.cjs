module.exports = {
  ci: {
    collect: {
      staticDistDir: './dist',
      url: ['/', '/docs/', '/benchmarks/', '/privacy/'],
      numberOfRuns: 1,
      settings: { preset: 'desktop' },
    },
    assert: {
      assertions: {
        'categories:accessibility': ['error', { minScore: 1 }],
        'categories:performance': ['warn', { minScore: 0.8 }],
      },
    },
    upload: { target: 'filesystem', outputDir: './lighthouse-report' },
  },
};
