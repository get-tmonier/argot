// Break: RxJS Observable/interval polling instead of Dagit's Apollo pollInterval / LiveDataProvider cadence.
// Dagit refreshes live data through Apollo useQuery pollInterval and the LiveDataProvider's
// LiveDataPollRateContext. RxJS Subjects, interval(), and pipe(switchMap(...)).subscribe() are a foreign
// reactive-streams runtime with no presence in ui-core.
import {interval, Subject} from 'rxjs';
import {switchMap, takeUntil} from 'rxjs/operators';

export function observeRunLogs(
  runId: string,
  fetchLogs: (runId: string) => Promise<string[]>,
  onBatch: (lines: string[]) => void,
): () => void {
  const stop$ = new Subject<void>();
  interval(2000)
    .pipe(
      takeUntil(stop$),
      switchMap(() => fetchLogs(runId)),
    )
    .subscribe((lines) => onBatch(lines));
  return () => {
    stop$.next();
    stop$.complete();
  };
}
