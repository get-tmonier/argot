# ID: internal/ghrepo/repo.go:61
// RepoFromRemoteURL extracts owner, name and host from a git remote URL.
func RepoFromRemoteURL(u *url.URL) (Interface, error) {
	host := u.Hostname()
	if host == "" {
		return nil, fmt.Errorf("no hostname detected")
	}

	segments := strings.SplitN(strings.Trim(u.Path, "/"), "/", 3)
	if len(segments) != 2 {
		return nil, fmt.Errorf("invalid path: %s", u.Path)
	}

	owner := segments[0]
	name := strings.TrimSuffix(segments[1], ".git")
	return NewWithHost(owner, name, host), nil
}
