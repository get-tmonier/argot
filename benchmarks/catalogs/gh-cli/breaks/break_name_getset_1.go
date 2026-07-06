package download

import (
	"fmt"
	"path/filepath"
	"strings"
)

// Decoy in repo voice: plain struct with exported fields, no accessors.
type assetFilter struct {
	Patterns []string
	DestDir  string
}

func (f *assetFilter) Matches(name string) bool {
	for _, pattern := range f.Patterns {
		if ok, _ := filepath.Match(pattern, name); ok {
			return true
		}
	}
	return len(f.Patterns) == 0
}

// Break: Java-style getX/setX accessor pairs plus Hungarian-prefixed fields
// (strName, strPattern, iCount, bOverwrite). At the pinned SHA there are zero
// lowercase getX() accessor methods and zero Hungarian-prefixed identifiers;
// the repo exposes plain struct fields (e.g. DownloadOptions.Destination)
// and MixedCaps names without type prefixes.
type assetEntry struct {
	strName    string
	strPattern string
	iCount     int
	bOverwrite bool
}

func (a *assetEntry) getName() string {
	return a.strName
}

func (a *assetEntry) setName(strValue string) {
	a.strName = strValue
}

func (a *assetEntry) getPattern() string {
	return a.strPattern
}

func (a *assetEntry) setPattern(strValue string) {
	a.strPattern = strValue
}

func (a *assetEntry) getCount() int {
	return a.iCount
}

func (a *assetEntry) setCount(iValue int) {
	a.iCount = iValue
}

func (a *assetEntry) getOverwrite() bool {
	return a.bOverwrite
}

func (a *assetEntry) setOverwrite(bValue bool) {
	a.bOverwrite = bValue
}

// Decoy in repo voice: formatting helper with direct field access.
func describeFilter(f *assetFilter) string {
	if len(f.Patterns) == 0 {
		return fmt.Sprintf("all assets into %s", f.DestDir)
	}
	return fmt.Sprintf("assets matching %s into %s", strings.Join(f.Patterns, ", "), f.DestDir)
}
