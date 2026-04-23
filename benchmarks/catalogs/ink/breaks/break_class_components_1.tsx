import React from 'react';
import { Text, Box } from 'ink';

// Break: class component in a hooks-only codebase.
export class Counter extends React.Component<{}, { count: number }> {
  state = { count: 0 };

  render() {
    return (
      <Box>
        <Text>Count: {this.state.count}</Text>
      </Box>
    );
  }
}
