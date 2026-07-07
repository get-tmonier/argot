import boxen from 'boxen';

// Break: boxen used to frame the "did you mean" suggestion in a terminal box —
// commander formats suggestions as plain strings; 'boxen' is 0-usage.
export function renderSuggestionBox(message) {
  return boxen(message, {
    padding: 1,
    borderColor: 'yellow',
    borderStyle: 'round',
  });
}
