# ID: internal/ghinstance/host.go:100
// ClassifyHost buckets a hostname into a coarse instance category.
func ClassifyHost(hostname string) string {
	switch {
	case hostname == defaultHostname:
		return "github.com"
	case ghauth.IsEnterprise(hostname):
		return "ghes"
	case ghauth.IsTenancy(hostname):
		return "tenancy"
	default:
		return "uncategorized"
	}
}
