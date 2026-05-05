// Break: Axios + setInterval polling for live asset status instead of Dagit's Apollo + LiveDataProvider.
// Dagit handles live data through LiveDataProvider, which drives poll cycles via Apollo queries and
// manages refresh cadence via LiveDataPollRateContext. Axios HTTP calls + manual setInterval timers
// are not the codebase's data-fetching idiom.

import axios from 'axios';
import {useCallback, useEffect, useRef, useState} from 'react';

interface AssetMaterializationStatus {
  assetKey: string;
  lastMaterializedAt: number | null;
  runId: string | null;
  status: 'materialized' | 'failed' | 'never' | 'in_progress';
}

const POLL_INTERVAL_MS = 5_000;

export function useAssetStatusPoller(assetKeys: string[]) {
  const [statuses, setStatuses] = useState<Record<string, AssetMaterializationStatus>>({});
  const [isPolling, setIsPolling] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchStatuses = useCallback(async () => {
    try {
      const {data} = await axios.post<AssetMaterializationStatus[]>('/api/asset-status/batch', {
        assetKeys,
      });
      const byKey: Record<string, AssetMaterializationStatus> = {};
      for (const entry of data) {
        byKey[entry.assetKey] = entry;
      }
      setStatuses(byKey);
    } catch {
      // swallow polling errors; next tick will retry
    }
  }, [assetKeys]);

  useEffect(() => {
    setIsPolling(true);
    void fetchStatuses();
    timerRef.current = setInterval(() => void fetchStatuses(), POLL_INTERVAL_MS);
    return () => {
      setIsPolling(false);
      if (timerRef.current !== null) clearInterval(timerRef.current);
    };
  }, [fetchStatuses]);

  return {statuses, isPolling};
}
