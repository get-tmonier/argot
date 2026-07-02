import * as React from "react";
import styled from "styled-components";

type Props = {
  documentId: string;
};

type State = {
  viewers: number;
};

// Break: class component with componentDidMount + this.setState in a hooks + observer codebase.
class ViewerCount extends React.Component<Props, State> {
  state: State = { viewers: 0 };
  interval?: ReturnType<typeof setInterval>;

  componentDidMount() {
    this.interval = setInterval(() => {
      this.setState((prev) => ({ viewers: prev.viewers }));
    }, 5000);
  }

  componentWillUnmount() {
    if (this.interval) {
      clearInterval(this.interval);
    }
  }

  render() {
    return <Count>{this.state.viewers} viewing</Count>;
  }
}

const Count = styled.span`
  font-size: 13px;
`;

export default ViewerCount;
