# ID: lib/help.js:418
function bucketByHeading(unsortedItems, visibleItems, getGroup) {
  const result = new Map();
  // Seed groups in order of appearance in unsortedItems.
  unsortedItems.forEach((item) => {
    const group = getGroup(item);
    if (!result.has(group)) result.set(group, []);
  });
  // Append items in order of appearance in visibleItems.
  visibleItems.forEach((item) => {
    const group = getGroup(item);
    if (!result.has(group)) {
      result.set(group, []);
    }
    result.get(group).push(item);
  });
  return result;
}
