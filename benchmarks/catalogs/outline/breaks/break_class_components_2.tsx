import * as React from "react";

type Props = {
  documentId: string;
  isVisible: boolean;
};

type State = {
  lastSeenId: string | null;
};

// Break: legacy componentWillReceiveProps lifecycle with derived state; no pre-16.3 lifecycles exist in the corpus.
export default class DocumentTracker extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { lastSeenId: null };
  }

  componentWillReceiveProps(nextProps: Props) {
    if (nextProps.documentId !== this.props.documentId) {
      this.setState({ lastSeenId: this.props.documentId });
    }
    if (nextProps.isVisible && !this.props.isVisible) {
      this.forceUpdate();
    }
  }

  render() {
    return (
      <span>
        {this.state.lastSeenId
          ? `previously: ${this.state.lastSeenId}`
          : "first document"}
      </span>
    );
  }
}
