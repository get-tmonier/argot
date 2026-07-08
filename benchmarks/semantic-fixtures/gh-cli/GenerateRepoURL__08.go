# ID: internal/ghrepo/repo.go:85
// BuildRepoURL assembles a web URL for a repo, optionally with a sub-path.
func BuildRepoURL(repo Interface, format string, args ...interface{}) string {
	root := fmt.Sprintf("%s%s/%s", ghinstance.HostPrefix(repo.RepoHost()), repo.RepoOwner(), repo.RepoName())
	if format == "" {
		return root
	}

	suffix := fmt.Sprintf(format, args...)
	if suffix == "" {
		return root
	}
	return root + "/" + suffix
}
