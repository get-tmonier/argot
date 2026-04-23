import React, { useEffect } from 'react';
import { Text, Box } from 'ink';
import $ from 'jquery';

export const Toggle = ({ selector, visible }: { selector: string; visible: boolean }) => {
  useEffect(() => {
    // Break: jQuery .hide()/.show() DOM manipulation called from a hook.
    if (visible) {
      $(selector).show();
    } else {
      $(selector).hide();
    }
  }, [selector, visible]);

  return (
    <Box>
      <Text>toggle {selector}: {visible ? 'on' : 'off'}</Text>
    </Box>
  );
};
