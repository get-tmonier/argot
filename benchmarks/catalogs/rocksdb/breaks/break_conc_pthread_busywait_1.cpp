// Break fixture -- not for compilation into the build.
#include "db/flush_job.h"
#include "monitoring/instrumented_mutex.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- Status return, mutex held by caller.
Status CheckFlushPreconditions(const autovector<MemTable*>& mems) {
  if (mems.empty()) {
    return Status::TryAgain("nothing to flush");
  }
  return Status::OK();
}

// Break: raw pthread + usleep busy-wait polling. rocksdb wraps threads in
// Break: port::Thread (used in db/compaction/compaction_job.cc itself) and
// Break: sleeps via Env SleepForMicroseconds / InstrumentedCondVar; zero
// Break: pthread_create or usleep calls in db/*.cc at the pinned SHA.
#include <pthread.h>
#include <unistd.h>

static volatile int flush_done_flag = 0;

static void* FlushWorkerMain(void* arg) {
  volatile int* flag = static_cast<volatile int*>(arg);
  usleep(2000);
  *flag = 1;
  return nullptr;
}

void RunFlushAndBusyWait() {
  pthread_t tid;
  flush_done_flag = 0;
  pthread_create(&tid, nullptr, &FlushWorkerMain,
                 const_cast<int*>(&flush_done_flag));
  while (flush_done_flag == 0) {
    usleep(100);  // spin until the worker signals completion
  }
  pthread_join(tid, nullptr);
}

}  // namespace ROCKSDB_NAMESPACE
