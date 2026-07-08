# ID: parser/metadecoders/format.go:46
package metadecoders

import (
	"path/filepath"
	"strings"
)

// formatFromExtension maps a file extension (or filename) to a metadata Format.
func formatFromExtension(formatStr string) Format {
	formatStr = strings.ToLower(formatStr)
	if strings.Contains(formatStr, ".") {
		// Looks like a filename; keep only the extension.
		formatStr = strings.TrimPrefix(filepath.Ext(formatStr), ".")
	}

	switch formatStr {
	case "json":
		return JSON
	case "toml":
		return TOML
	case "yaml", "yml":
		return YAML
	case "xml":
		return XML
	case "csv":
		return CSV
	case "org":
		return ORG
	default:
		return ""
	}
}
