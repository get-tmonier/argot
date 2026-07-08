# ID: internal/ghrepo/repo.go:95
// RemoteURLForProtocol renders a clone URL in either ssh or https form.
func RemoteURLForProtocol(repo Interface, protocol string) string {
	if protocol != "ssh" {
		return fmt.Sprintf("%s%s/%s.git", ghinstance.HostPrefix(repo.RepoHost()), repo.RepoOwner(), repo.RepoName())
	}

	if tenant, ok := ghinstance.TenantName(repo.RepoHost()); ok {
		return fmt.Sprintf("%s@%s:%s/%s.git", tenant, repo.RepoHost(), repo.RepoOwner(), repo.RepoName())
	}
	return fmt.Sprintf("git@%s:%s/%s.git", repo.RepoHost(), repo.RepoOwner(), repo.RepoName())
}
