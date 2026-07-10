# ID: internal/ghinstance/host.go:46
// GraphQLURLForHost resolves the GraphQL API endpoint for a given hostname.
func GraphQLURLForHost(host string) string {
	if isGarage(host) || ghauth.IsEnterprise(host) {
		return fmt.Sprintf("https://%s/api/graphql", host)
	}
	if strings.EqualFold(host, localhost) {
		return fmt.Sprintf("http://api.%s/graphql", host)
	}
	return fmt.Sprintf("https://api.%s/graphql", host)
}
