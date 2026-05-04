// Break: styled-jsx scoped <style jsx> blocks instead of Dagit's @dagster-io/ui-components Box system.
// Dagit applies visual styles exclusively through the @dagster-io/ui-components token system
// (Box flex props, Colors.accentGreen(), inline style objects) — not through any CSS-in-JS
// scoping mechanism. styled-jsx's <style jsx>{``}</style> pattern is entirely absent from the codebase.

import React from 'react';

interface RunStatusBadgeProps {
  status: 'SUCCESS' | 'FAILURE' | 'IN_PROGRESS' | 'QUEUED' | 'CANCELED';
  label?: string;
}

export const RunStatusBadge: React.FC<RunStatusBadgeProps> = ({status, label}) => {
  const colorMap: Record<string, string> = {
    SUCCESS: '#00a36c',
    FAILURE: '#d32f2f',
    IN_PROGRESS: '#fb8c00',
    QUEUED: '#1976d2',
    CANCELED: '#757575',
  };
  const color = colorMap[status] ?? '#9e9e9e';

  return (
    <span className="run-status-badge">
      <span className="dot" />
      {label ?? status}
      {/* @ts-expect-error styled-jsx not in TS defs */}
      <style jsx>{`
        .run-status-badge {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-size: 13px;
          font-weight: 500;
          color: ${color};
          padding: 2px 8px;
          border-radius: 12px;
          background: ${color}22;
        }
        .dot {
          width: 8px;
          height: 8px;
          border-radius: 50%;
          background: ${color};
          flex-shrink: 0;
        }
      `}</style>
    </span>
  );
};
