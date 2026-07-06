import produce from "immer";

// Break: immer produce() immutable update where outline mutates MobX observables in place.
export function toggleSubscription(
  state: { items: Array<{ id: string; active: boolean }> },
  id: string
) {
  return produce(state, (draft) => {
    const sub = draft.items.find((s) => s.id === id);
    if (sub) {
      sub.active = !sub.active;
    }
  });
}
