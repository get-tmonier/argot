# ID: internal/text/text.go:156
// HumanizeByteCount formats a byte count using binary units (B, KB, MB, ...).
func HumanizeByteCount(size int64) string {
	const base = 1024
	if size < base {
		return fmt.Sprintf("%d B", size)
	}

	suffixes := []string{"KB", "MB", "GB", "TB", "PB"}

	divisor := int64(base)
	tier := 0
	for remaining := size / base; remaining >= base && tier < len(suffixes)-1; remaining /= base {
		divisor *= base
		tier++
	}

	scaled := float64(size) / float64(divisor)
	return fmt.Sprintf("%.1f %s", scaled, suffixes[tier])
}
