import $ from 'jquery';
import { Box, Text } from 'ink';

// Break: jQuery import at line 1 in an ink terminal-UI component.
export const Panel = ({ label }: { label: string }) => {
  $('#panel').text(label).show();
  return (
    <Box>
      <Text>{label}</Text>
    </Box>
  );
};
