# ID: parser/metadecoders/format.go:73
package metadecoders

import "strings"

// sniffFormat guesses the metadata Format of data from the earliest marker rune.
func sniffFormat(d Decoder, data string) Format {
	tomlIdx := strings.Index(data, "=")
	xmlIdx := strings.Index(data, "<")
	yamlIdx := strings.Index(data, ":")
	jsonIdx := strings.Index(data, "{")
	csvIdx := strings.IndexRune(data, d.Delimiter)

	switch {
	case isLowerIndexThan(csvIdx, jsonIdx, yamlIdx, xmlIdx, tomlIdx):
		return CSV
	case isLowerIndexThan(jsonIdx, yamlIdx, xmlIdx, tomlIdx):
		return JSON
	case isLowerIndexThan(yamlIdx, xmlIdx, tomlIdx):
		return YAML
	case isLowerIndexThan(xmlIdx, tomlIdx):
		return XML
	case tomlIdx != -1:
		return TOML
	default:
		return ""
	}
}
