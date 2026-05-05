// Break: React Router v5 Switch/Route/component= declarative routing instead of Dagit's Next.js pages.
// Dagit is a Next.js application; page routing is handled via the Next.js pages/ directory and
// useRouter() from 'next/router'. React Router v5's <Switch>, <Route component={...}>, and
// useRouteMatch are absent — they conflict with the Next.js routing model.

import React from 'react';
import {Route, Switch, useRouteMatch} from 'react-router-dom';

interface MatchParams {
  group: string;
  assetName: string;
}

function AssetDetailPage({match}: {match: {params: MatchParams}}) {
  const {assetGroup, assetName} = match.params as unknown as {assetGroup: string; assetName: string};
  return (
    <div className="asset-detail">
      <h2>
        {assetGroup} / {assetName}
      </h2>
    </div>
  );
}

function AssetGroupPage({match}: {match: {params: {group: string}}}) {
  return (
    <div className="asset-group">
      <h2>Group: {match.params.group}</h2>
    </div>
  );
}

export function AssetRouter() {
  const {path} = useRouteMatch();
  return (
    <Switch>
      <Route exact path={path} component={AssetGroupPage} />
      <Route path={`${path}/:group/:assetName`} component={AssetDetailPage} />
      <Route path={`${path}/:group`} component={AssetGroupPage} />
    </Switch>
  );
}

export function AssetsApp() {
  return (
    <div className="assets-app">
      <AssetRouter />
    </div>
  );
}
