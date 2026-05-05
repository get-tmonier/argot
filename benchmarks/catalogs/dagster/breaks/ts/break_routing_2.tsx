// Break: React Router v5 withRouter HOC + history.push navigation in a class component.
// Dagit navigates programmatically using useRouter() from 'next/router' inside function components.
// The withRouter HOC, history.push, and class-component-based navigation are React Router v5 patterns
// with no analog in Dagit's Next.js codebase.

import React, {Component} from 'react';
import {RouteComponentProps, withRouter} from 'react-router-dom';

interface OwnProps {
  runId: string;
}

type Props = OwnProps & Partial<RouteComponentProps>;

class RunNavigatorBase extends Component<Props> {
  handleViewLogs = () => {
    const {history, runId} = this.props;
    history?.push(`/runs/${runId}/logs`);
  };

  handleViewTimeline = () => {
    const {history, runId} = this.props;
    history?.push(`/runs/${runId}/timeline`);
  };

  handleViewStepEvents = () => {
    const {history, runId} = this.props;
    history?.push(`/runs/${runId}/steps`);
  };

  render() {
    const {runId} = this.props;
    return (
      <div className="run-nav">
        <span className="run-id">{runId}</span>
        <button onClick={this.handleViewLogs}>View Logs</button>
        <button onClick={this.handleViewTimeline}>View Timeline</button>
        <button onClick={this.handleViewStepEvents}>View Steps</button>
      </div>
    );
  }
}

export const RunNavigator = withRouter(RunNavigatorBase as React.ComponentType<RouteComponentProps>);
