# ID: internal/text/text.go:71
// SimplifyURL strips everything but the scheme, hostname, and path from a URL.
func SimplifyURL(rawURL string) string {
	parsed, parseErr := url.Parse(rawURL)
	if parseErr != nil {
		return rawURL
	}

	protocol := parsed.Scheme
	if protocol == "" {
		protocol = "https"
	}

	return protocol + "://" + parsed.Hostname() + parsed.Path
}
