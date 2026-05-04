// Break: MobX observable store + @observer decorator instead of Dagit's Recoil atoms + useRecoilState.
// Dagit uses Recoil (RecoilRoot wraps the app; atom/selector/useRecoilState for shared state).
// MobX's makeAutoObservable, observer, and class-based stores are not present anywhere in the UI codebase.

import {makeAutoObservable} from 'mobx';
import {observer} from 'mobx-react-lite';
import React from 'react';

class AssetSelectionStore {
  selectedKeys: string[] = [];
  filterQuery = '';

  constructor() {
    makeAutoObservable(this);
  }

  toggleKey(key: string) {
    const idx = this.selectedKeys.indexOf(key);
    if (idx >= 0) {
      this.selectedKeys.splice(idx, 1);
    } else {
      this.selectedKeys.push(key);
    }
  }

  setFilter(query: string) {
    this.filterQuery = query;
  }

  get selectionCount() {
    return this.selectedKeys.length;
  }

  get hasSelection() {
    return this.selectedKeys.length > 0;
  }
}

export const selectionStore = new AssetSelectionStore();

export const AssetSelectionPanel = observer(() => {
  return (
    <div>
      <input
        value={selectionStore.filterQuery}
        onChange={(e) => selectionStore.setFilter(e.target.value)}
        placeholder="Filter assets..."
      />
      <p>
        {selectionStore.selectionCount} asset{selectionStore.selectionCount !== 1 ? 's' : ''}{' '}
        selected
      </p>
      {selectionStore.hasSelection && (
        <button onClick={() => selectionStore.toggleKey(selectionStore.selectedKeys[0]!)}>
          Deselect first
        </button>
      )}
    </div>
  );
});
