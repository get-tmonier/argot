import React from 'react';
import { Text, Box } from 'ink';

type Props = { value: number };
type State = { cached: number };

export class Mirror extends React.Component<Props, State> {
  state: State = { cached: this.props.value };

  // Break: legacy componentWillReceiveProps lifecycle in an ink file.
  componentWillReceiveProps(nextProps: Props) {
    if (nextProps.value !== this.props.value) {
      this.setState({ cached: nextProps.value });
    }
  }

  render() {
    return (
      <Box>
        <Text>cached={this.state.cached}</Text>
      </Box>
    );
  }
}
