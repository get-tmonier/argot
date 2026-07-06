import { Subject } from "rxjs";
import { debounceTime } from "rxjs/operators";

// Break: rxjs reactive Subject + operators pipeline for scroll events where
// excalidraw handles events with plain callbacks and its own throttle; rxjs
// is 0-usage at the pinned SHA and absent from package.json.
export const createScrollStream = (
  onScroll: (position: { x: number; y: number }) => void,
) => {
  const scroll$ = new Subject<{ x: number; y: number }>();
  scroll$.pipe(debounceTime(16)).subscribe(onScroll);
  return scroll$;
};
