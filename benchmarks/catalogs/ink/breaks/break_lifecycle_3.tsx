import React from 'react';
import { Text, Box } from 'ink';

type State = { ticks: number };

export class Watcher extends React.Component<{}, State> {
  state: State = { ticks: 0 };
  private timer: NodeJS.Timeout | null = null;

  // Break: componentWillUnmount cleanup instead of useEffect return fn.
  componentWillUnmount() {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  render() {
    return (
      <Box>
        <Text>ticks={this.state.ticks}</Text>
      </Box>
    );
  }
}
