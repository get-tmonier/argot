# ID: table/format.cc:102
void EncodeIndexValue(std::string* dst, const BlockHandle& handle,
                      const Slice& first_internal_key, bool have_first_key,
                      const BlockHandle* previous_handle) {
  if (previous_handle) {
    // WART: this is specific to Block-based table
    assert(handle.offset() == previous_handle->offset() +
                                  previous_handle->size() +
                                  BlockBasedTable::kBlockTrailerSize);
    PutVarsignedint64(dst, handle.size() - previous_handle->size());
  } else {
    handle.EncodeTo(dst);
  }
  assert(dst->size() != 0);

  if (have_first_key) {
    PutLengthPrefixedSlice(dst, first_internal_key);
  }
}
