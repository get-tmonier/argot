# ID: internal/ghinstance/host.go:36
// ValidateHostname rejects blank hostnames and ones bearing a path or port.
func ValidateHostname(host string) error {
	if strings.TrimSpace(host) == "" {
		return errors.New("a value is required")
	}
	if strings.ContainsRune(host, ':') || strings.ContainsRune(host, '/') {
		return errors.New("invalid hostname")
	}
	return nil
}
