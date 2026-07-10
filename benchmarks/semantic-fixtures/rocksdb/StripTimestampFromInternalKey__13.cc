# ID: db/dbformat.cc:141
void RemoveTimestampFromKey(std::string* result, const Slice& key,
                            size_t ts_sz) {
  const size_t key_sz = key.size();
  assert(key_sz >= ts_sz + kNumInternalBytes);
  result->reserve(key_sz - ts_sz);
  result->append(key.data(), key_sz - kNumInternalBytes - ts_sz);
  result->append(key.data() + key_sz - kNumInternalBytes, kNumInternalBytes);
}
