# ID: helpers/general.go:200
package helpers

import "fmt"

// humanizeByteSize pretty-prints a byte count using binary GB/MB/KB units.
func humanizeByteSize(bc uint64) string {
	const (
		Kilobyte = 1 << 10
		Megabyte = 1 << 20
		Gigabyte = 1 << 30
	)

	switch {
	case bc > Gigabyte || -bc > Gigabyte:
		return fmt.Sprintf("%.2f GB", float64(bc)/Gigabyte)
	case bc > Megabyte || -bc > Megabyte:
		return fmt.Sprintf("%.2f MB", float64(bc)/Megabyte)
	case bc > Kilobyte || -bc > Kilobyte:
		return fmt.Sprintf("%.2f KB", float64(bc)/Kilobyte)
	default:
		return fmt.Sprintf("%d B", bc)
	}
}
