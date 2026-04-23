import React from 'react';
import { Text, Box } from 'ink';

export const Panel = ({ title, broken }: { title: string; broken: boolean }) => {
  return (
    <Box>
      {/* Break: try-block with no catch that throws from render. */}
      {(() => {
        try {
          if (broken) throw new Error(`panel ${title} is broken`);
          return <Text>{title}</Text>;
        } finally {
          // no-op
        }
      })()}
    </Box>
  );
};
