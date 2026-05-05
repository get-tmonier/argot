// Break: Redux connect() HOC + class component instead of Dagit's function components + Recoil atoms.
// Dagit uses function components throughout, useRecoilState/useRecoilValue for shared state,
// and custom domain hooks. Redux + class components and the connect HOC are absent from the codebase.

import React, {Component} from 'react';
import {connect} from 'react-redux';

interface StateProps {
  assetGroups: string[];
  isLoading: boolean;
}

interface DispatchProps {
  fetchAssetGroups: () => void;
}

type Props = StateProps & DispatchProps;

class AssetGroupsPanel extends Component<Props> {
  componentDidMount() {
    this.props.fetchAssetGroups();
  }

  render() {
    const {assetGroups, isLoading} = this.props;
    if (isLoading) {
      return <div className="loading-spinner" />;
    }
    return (
      <div className="asset-groups-panel">
        <h2>Asset Groups</h2>
        <ul>
          {assetGroups.map((group) => (
            <li key={group}>{group}</li>
          ))}
        </ul>
      </div>
    );
  }
}

const mapStateToProps = (state: {assets: {groups: string[]; loading: boolean}}): StateProps => ({
  assetGroups: state.assets.groups,
  isLoading: state.assets.loading,
});

const mapDispatchToProps = (dispatch: (a: {type: string}) => void): DispatchProps => ({
  fetchAssetGroups: () => dispatch({type: 'FETCH_ASSET_GROUPS'}),
});

export default connect(mapStateToProps, mapDispatchToProps)(AssetGroupsPanel);
