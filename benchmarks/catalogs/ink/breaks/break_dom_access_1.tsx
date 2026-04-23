import React from 'react';
import { Text } from 'ink';

export const StatusBar = () => {
  // Break: document.* / window.* in a terminal UI has no meaning.
  const title = document.getElementById('title')?.textContent ?? 'ink';
  window.addEventListener('resize', () => {});
  return <Text>{title}</Text>;
};
