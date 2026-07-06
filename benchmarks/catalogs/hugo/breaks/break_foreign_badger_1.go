package filecache

import (
	"strings"
)

// Decoy in repo voice: cache-id guard matching the filecache package.
func cacheIDValid(id string) bool {
	return id != "" && !strings.HasPrefix(id, "/")
}

// Break: uses github.com/dgraph-io/badger/v4 (with the badger/v4/options
// submodule) to back the file cache with an embedded LSM key-value store.
// At the pinned SHA dgraph-io/badger appears in zero .go files and is absent
// from go.mod; Hugo's caches persist to the local filesystem through
// github.com/spf13/afero, never an embedded database engine.
import (
	badger "github.com/dgraph-io/badger/v4"
	"github.com/dgraph-io/badger/v4/options"
)

func openBadgerCache(dir string) (*badger.DB, error) {
	opts := badger.DefaultOptions(dir).WithCompression(options.ZSTD)
	return badger.Open(opts)
}
