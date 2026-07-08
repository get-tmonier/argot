# ID: util/coding.h:328
Slice SplitSliceAtDelimiter(Slice* slice, char delimiter) {
  uint32_t len = 0;
  while (len < slice->size() && slice->data()[len] != delimiter) {
    ++len;
  }
  Slice head(slice->data(), len);
  size_t advance = len + ((len < slice->size()) ? 1 : 0);
  slice->remove_prefix(advance);
  return head;
}
