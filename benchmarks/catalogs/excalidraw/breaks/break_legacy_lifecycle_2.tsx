import React from "react";

import type { ExcalidrawElement } from "@excalidraw/element/types";

type Props = { elements: readonly ExcalidrawElement[] };
type State = { dimensionLabel: string };

// Break: UNSAFE_ legacy lifecycle pair driving derived state, where this
// codebase derives values inline in function components with useMemo.
export class SelectedShapeStats extends React.Component<Props, State> {
  state: State = { dimensionLabel: "" };

  UNSAFE_componentWillMount() {
    this.recomputeLabel(this.props.elements);
  }

  UNSAFE_componentWillUpdate(nextProps: Props) {
    if (nextProps.elements !== this.props.elements) {
      this.recomputeLabel(nextProps.elements);
    }
  }

  recomputeLabel(elements: readonly ExcalidrawElement[]) {
    const [first] = elements;
    this.setState({
      dimensionLabel: first
        ? `${Math.round(first.width)} × ${Math.round(first.height)}`
        : "",
    });
  }

  render() {
    return <div className="Stats__dimension">{this.state.dimensionLabel}</div>;
  }
}
