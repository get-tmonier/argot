import React from 'react';
import { Text, Box } from 'ink';
import $ from 'jquery';

export const Menu = ({ items }: { items: string[] }) => {
  // Break: jQuery selector + click binding in an ink functional component.
  $('.item').on('click', (event) => {
    console.log('clicked', event.target);
  });

  return (
    <Box flexDirection="column">
      {items.map((item) => (
        <Text key={item}>{item}</Text>
      ))}
    </Box>
  );
};
