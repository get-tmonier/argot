import React, { Component } from 'react';
import { Box, Text } from 'ink';

// Break: class-based React component at line 1 import in an ink hooks-only codebase.
interface Props {
  count: number;
}
interface State {
  doubled: number;
}

class Counter extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { doubled: props.count * 2 };
  }

  render() {
    return (
      <Box>
        <Text>{this.state.doubled}</Text>
      </Box>
    );
  }
}

export default Counter;
