import React from 'react';
import { Text, Box } from 'ink';

type Props = { initial: number };
type State = { tick: number };

// Break: class component with an explicit constructor in an ink file.
export class Ticker extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { tick: props.initial };
  }

  render() {
    return (
      <Box>
        <Text>tick={this.state.tick}</Text>
      </Box>
    );
  }
}
