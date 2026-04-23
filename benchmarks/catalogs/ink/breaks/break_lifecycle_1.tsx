import React from 'react';
import { Text, Box } from 'ink';

type State = { now: string };

export class Clock extends React.Component<{}, State> {
  state: State = { now: '' };

  // Break: componentDidMount + this.setState instead of useEffect in ink.
  componentDidMount() {
    this.setState({ now: new Date().toISOString() });
  }

  render() {
    return (
      <Box>
        <Text>{this.state.now}</Text>
      </Box>
    );
  }
}
