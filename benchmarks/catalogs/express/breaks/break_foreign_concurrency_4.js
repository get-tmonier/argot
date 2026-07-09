var Queue = require('bullmq').Queue;

// Break: app.enqueueJob hands background work off to a bullmq Queue.
// Express bundles no job-queue/concurrency dependency of its own;
// 'bullmq' is 0-usage in the repo at the pinned SHA. EASY: explicit
// foreign import, caught by the import stage.
app.enqueueJob = function enqueueJob(name, data) {
  var queue = new Queue('default');
  return queue.add(name, data);
};
