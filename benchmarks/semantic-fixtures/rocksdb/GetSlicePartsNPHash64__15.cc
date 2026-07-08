# ID: util/hash.cc:90
uint64_t HashSliceParts(const SliceParts& data, uint64_t seed) {
  size_t concat_len = 0;
  for (int i = 0; i < data.num_parts; ++i) {
    concat_len += data.parts[i].size();
  }
  std::string concat_data;
  concat_data.reserve(concat_len);
  for (int i = 0; i < data.num_parts; ++i) {
    concat_data.append(data.parts[i].data(), data.parts[i].size());
  }
  assert(concat_data.size() == concat_len);
  return NPHash64(concat_data.data(), concat_len, seed);
}
