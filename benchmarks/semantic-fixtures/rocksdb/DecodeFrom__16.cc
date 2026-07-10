# ID: table/format.cc:66
Status ReadBlockHandle(Slice* input, uint64_t* offset, uint64_t* size) {
  if (GetVarint64(input, offset) && GetVarint64(input, size)) {
    return Status::OK();
  }
  // reset in case of failure after partially decoding
  *offset = 0;
  *size = 0;
  return Status::Corruption("bad block handle");
}
