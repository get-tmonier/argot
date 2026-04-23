import React, { useState, useEffect } from 'react';
import { Text, Box } from 'ink';

export const Preferences = () => {
  const [theme, setTheme] = useState('dark');

  useEffect(() => {
    // Break: localStorage is a browser API, unavailable in an ink/node runtime.
    const saved = localStorage.getItem('theme');
    if (saved) setTheme(saved);
  }, []);

  return (
    <Box>
      <Text>theme: {theme}</Text>
    </Box>
  );
};
