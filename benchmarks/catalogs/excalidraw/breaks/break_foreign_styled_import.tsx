import React from "react";
import styled from "styled-components";

// Break: styled-components CSS-in-JS for a toast where excalidraw styles
// with SCSS class names; styled-components is 0-usage at the pinned SHA and
// absent from package.json.
const ToastCard = styled.div`
  position: absolute;
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--island-bg-color);
`;

export const StyledToast = ({ message }: { message: string }) => (
  <ToastCard className="toast">{message}</ToastCard>
);
