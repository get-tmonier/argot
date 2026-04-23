import React, { Component } from 'react';
import { Text, Box } from 'ink';

type Props = { label: string };
type State = { value: number };

// Break: `extends Component` class with this.props / this.state in an ink file.
export class Gauge extends Component<Props, State> {
  state: State = { value: 0 };

  render() {
    return (
      <Box>
        <Text>{this.props.label}: {this.state.value}</Text>
      </Box>
    );
  }
}
