import * as React from "react";
import { connect } from "react-redux";
import { useSelector, useDispatch } from "react-redux";

type StateProps = {
  unreadCount: number;
};

type DispatchProps = {
  markAllRead: () => void;
};

// Break: react-redux connect() + mapStateToProps boilerplate where the voice is useStores() + observer.
function NotificationBadge({ unreadCount, markAllRead }: StateProps & DispatchProps) {
  const isOpen = useSelector((state: any) => state.notifications.isOpen);
  const dispatch = useDispatch();

  return (
    <button
      onClick={() => {
        markAllRead();
        dispatch({ type: "notifications/close" });
      }}
    >
      {isOpen ? "Close" : `${unreadCount} unread`}
    </button>
  );
}

const mapStateToProps = (state: any): StateProps => ({
  unreadCount: state.notifications.items.filter((n: any) => !n.read).length,
});

const mapDispatchToProps = (dispatch: any): DispatchProps => ({
  markAllRead: () => dispatch({ type: "notifications/markAllRead" }),
});

export default connect(mapStateToProps, mapDispatchToProps)(NotificationBadge);
