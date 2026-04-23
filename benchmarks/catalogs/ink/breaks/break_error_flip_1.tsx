import React from 'react';
import { Text, Box } from 'ink';

export const Guard = ({ bad, label }: { bad: boolean; label: string }) => {
  return (
    <Box>
      {/* Break: throwing inside render instead of rendering an error node. */}
      {(() => {
        if (bad) throw new Error('nope');
        return <Text>{label}</Text>;
      })()}
    </Box>
  );
};
