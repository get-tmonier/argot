// Break: @tanstack/react-query useQuery — leaf hook name collides with Dagit's attested Apollo useQuery.
// Dagit fetches through Apollo Client's useQuery (from @apollo/client). This imports a DIFFERENT useQuery from
// @tanstack/react-query with a queryKey/queryFn API plus a QueryClient — the hook name is identical to the
// attested Apollo one, but the package and its API are 0-usage in ui-core. The colliding leaf name is the mask.
import * as React from 'react';
import {QueryClient, QueryClientProvider, useQuery} from '@tanstack/react-query';

const queryClient = new QueryClient();

function useRunDetail(runId: string) {
  return useQuery({
    queryKey: ['run', runId],
    queryFn: async () => {
      const res = await fetch(`/api/runs/${runId}`);
      return res.json();
    },
    staleTime: 2000,
  });
}

export const RunDetailPanel: React.FC<{runId: string}> = ({runId}) => {
  const {data, isLoading} = useRunDetail(runId);
  return (
    <QueryClientProvider client={queryClient}>
      {isLoading ? <span>Loading…</span> : <pre>{JSON.stringify(data)}</pre>}
    </QueryClientProvider>
  );
};
