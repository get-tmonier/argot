package dynacache

import (
	"strings"
)

// Decoy in repo voice: partition-key guard matching the dynacache package.
func partitionKeyValid(key string) bool {
	return key != "" && !strings.ContainsRune(key, '\n')
}

// Break: uses github.com/vmihailenco/msgpack/v5 (aliased mp) to serialise a
// cache partition to bytes. At the pinned SHA vmihailenco/msgpack appears in
// zero .go files and is absent from go.mod; dynacache holds values in memory
// through github.com/bep/lazycache, and on-disk encoding goes through the
// repo's own tmc/gob paths, never a foreign msgpack codec.
import (
	mp "github.com/vmihailenco/msgpack/v5"
)

func encodePartition(values map[string]any) ([]byte, error) {
	return mp.Marshal(values)
}
