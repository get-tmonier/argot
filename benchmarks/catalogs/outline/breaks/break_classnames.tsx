import * as React from "react";
import classNames from "classnames";

// Break: classnames() className composition where outline styles with styled-components.
export function BreadcrumbLabel({
  active,
  archived,
  children,
}: {
  active: boolean;
  archived: boolean;
  children: React.ReactNode;
}) {
  const cls = classNames("breadcrumb-label", {
    "is-active": active,
    "is-archived": archived,
  });
  return <span className={cls}>{children}</span>;
}
