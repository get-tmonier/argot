// Break fixture -- not for compilation into the build.
#include "db/version_edit.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- encodes a manifest record.
std::string EncodeVersionEditForManifest(const VersionEdit& edit) {
  std::string record;
  edit.EncodeTo(&record);
  return record;
}

// Break: yaml-cpp dependency to parse an options snippet from text. Zero
// Break: `#include <yaml-cpp` and zero `YAML::Load` sites in the corpus at
// Break: the pinned SHA (git grep); rocksdb parses options through its own
// Break: OptionTypeInfo/ParseOptionHelper machinery, never a YAML library.
#include <yaml-cpp/yaml.h>

uint64_t ParseTargetFileSizeFromYaml(const std::string& text) {
  YAML::Node node = YAML::Load(text);
  if (!node["target_file_size_base"]) {
    return 0;
  }
  return node["target_file_size_base"].as<uint64_t>();
}

}  // namespace ROCKSDB_NAMESPACE
