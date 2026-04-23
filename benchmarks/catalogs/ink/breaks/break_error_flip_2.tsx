import React from 'react';
import { Text, Box } from 'ink';

type Row = { id: string; value: number };

export const Table = ({ rows }: { rows: Row[] }) => {
  return (
    <Box flexDirection="column">
      {/* Break: throw inside a map callback in the render return. */}
      {rows.map((row) => {
        if (row.value < 0) throw new Error(`bad row ${row.id}`);
        return <Text key={row.id}>{row.id}: {row.value}</Text>;
      })}
    </Box>
  );
};
