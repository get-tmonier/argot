// Break: CSS Modules + classnames utility instead of Dagit's @dagster-io/ui-components design system.
// Dagit styles every UI element using Box, Colors, Icon, and other primitives from @dagster-io/ui-components,
// which wraps a token-driven design system (flex props, inline style overrides via the `style` prop).
// CSS module imports (*.module.css) and the classnames package are not used in the Dagit codebase.

import classNames from 'classnames';
import React, {useState} from 'react';

import styles from './AssetGroupCard.module.css';

interface AssetGroupCardProps {
  groupName: string;
  assetCount: number;
  isSelected?: boolean;
  onClick?: () => void;
}

export const AssetGroupCard: React.FC<AssetGroupCardProps> = ({
  groupName,
  assetCount,
  isSelected = false,
  onClick,
}) => {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      className={classNames(styles.card, {
        [styles.selected!]: isSelected,
        [styles.hovered!]: isHovered,
      })}
      onClick={onClick}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      role="button"
      tabIndex={0}
    >
      <span className={styles.groupName}>{groupName}</span>
      <span
        className={classNames(styles.badge, {
          [styles.badgeActive!]: assetCount > 0,
        })}
      >
        {assetCount}
      </span>
    </div>
  );
};
