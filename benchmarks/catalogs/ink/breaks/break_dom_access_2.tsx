import React from 'react';
import { Text, Box, useInput } from 'ink';

export const Redirector = ({ target }: { target: string }) => {
  useInput((input) => {
    if (input === 'g') {
      // Break: window.location.href navigation in a terminal UI.
      window.location.href = target;
    }
  });

  return (
    <Box>
      <Text>press g to go to {target}</Text>
    </Box>
  );
};
