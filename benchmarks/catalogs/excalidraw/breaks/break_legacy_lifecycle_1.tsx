import React from "react";

import type { UIAppState } from "../types";

type Props = { appState: UIAppState };
type State = { visible: boolean };

// Break: legacy componentWillReceiveProps / componentWillMount lifecycle in
// a codebase whose components are functional with hooks (the one class
// component, App, uses only modern lifecycle methods).
class OverwriteConfirmBanner extends React.Component<Props, State> {
  state: State = { visible: false };

  componentWillMount() {
    this.setState({ visible: this.props.appState.openDialog !== null });
  }

  componentWillReceiveProps(nextProps: Props) {
    if (nextProps.appState.openDialog !== this.props.appState.openDialog) {
      this.setState({ visible: nextProps.appState.openDialog !== null });
    }
  }

  render() {
    if (!this.state.visible) {
      return null;
    }
    return (
      <div className="OverwriteConfirm__banner">
        Unsaved changes will be overwritten.
      </div>
    );
  }
}

export default OverwriteConfirmBanner;
