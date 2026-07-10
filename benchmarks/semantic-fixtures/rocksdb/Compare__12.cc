# ID: db/dbformat.cc:199
int CompareInternalKeys(const Comparator& user_comparator,
                        const ParsedInternalKey& a,
                        const ParsedInternalKey& b) {
  // Order by increasing user key, then decreasing sequence number,
  // then decreasing type.
  int r = user_comparator.Compare(a.user_key, b.user_key);
  if (r != 0) {
    return r;
  }
  if (a.sequence > b.sequence) {
    return -1;
  } else if (a.sequence < b.sequence) {
    return +1;
  } else if (a.type > b.type) {
    return -1;
  } else if (a.type < b.type) {
    return +1;
  }
  return 0;
}
