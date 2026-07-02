import * as React from "react";
import styled from "styled-components";
import { s } from "@shared/styles";

type Props = {
  documentId: string;
  isCollapsed: boolean;
};

function CollapsibleSection({ documentId, isCollapsed }: Props) {
  React.useEffect(() => {
    // Break: jQuery DOM mutation inside a React component in a React + styled-components codebase.
    if (isCollapsed) {
      $(`#section-${documentId}`).slideUp(200).addClass("collapsed");
      $(".section-toggle").attr("aria-expanded", "false");
    } else {
      $(`#section-${documentId}`).slideDown(200).removeClass("collapsed");
      $(".section-toggle").attr("aria-expanded", "true");
    }
  }, [documentId, isCollapsed]);

  return (
    <Wrapper id={`section-${documentId}`}>
      <button className="section-toggle">Toggle</button>
    </Wrapper>
  );
}

const Wrapper = styled.div`
  border-radius: 4px;
  background: ${s("background")};
`;

export default CollapsibleSection;
