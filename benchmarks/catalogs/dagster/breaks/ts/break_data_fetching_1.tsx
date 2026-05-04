// Break: raw fetch() + useEffect + manual loading/error state instead of Dagit's Apollo useQuery.
// Dagit fetches all data via Apollo Client (useQuery/useLazyQuery wrapping GraphQL operations).
// Hand-rolled REST fetch with manual {data, loading, error} state triples is not the Dagit pattern;
// the codebase has no fetch() calls wiring UI state outside of Apollo.

import React, {useEffect, useState} from 'react';

interface RunRecord {
  id: string;
  status: string;
  startTime: number | null;
  jobName: string;
}

interface FetchState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

export function useRunHistory(repositoryId: string): FetchState<RunRecord[]> {
  const [state, setState] = useState<FetchState<RunRecord[]>>({
    data: null,
    loading: true,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;
    setState({data: null, loading: true, error: null});

    fetch(`/api/runs?repositoryId=${encodeURIComponent(repositoryId)}`)
      .then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.json() as Promise<RunRecord[]>;
      })
      .then((data) => {
        if (!cancelled) setState({data, loading: false, error: null});
      })
      .catch((err: unknown) => {
        if (!cancelled) setState({data: null, loading: false, error: String(err)});
      });

    return () => {
      cancelled = true;
    };
  }, [repositoryId]);

  return state;
}

export const RunHistoryPanel: React.FC<{repositoryId: string}> = ({repositoryId}) => {
  const {data, loading, error} = useRunHistory(repositoryId);
  if (loading) return <div>Loading…</div>;
  if (error) return <div>Error: {error}</div>;
  return (
    <ul>
      {(data ?? []).map((run) => (
        <li key={run.id}>
          {run.jobName}: {run.status}
        </li>
      ))}
    </ul>
  );
};
