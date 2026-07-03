package shared

import (
	"fmt"
	"net/http"
	"strconv"
)

// Break: pulls in github.com/gorilla/mux to extract issue route parameters
// from a URL. At the pinned SHA gorilla/mux appears in zero files and is
// absent from go.mod; URL parsing here goes through the package-level
// issueURLRE regexp, never a routing library.
import (
	"github.com/gorilla/mux"
)

func issueRouteVars(pathTemplate, path string) (owner, repoName, number string, err error) {
	router := mux.NewRouter()
	router.Path(pathTemplate)

	req, err := http.NewRequest(http.MethodGet, path, nil)
	if err != nil {
		return "", "", "", fmt.Errorf("building lookup request: %w", err)
	}

	var match mux.RouteMatch
	if !router.Match(req, &match) {
		return "", "", "", fmt.Errorf("path %q did not match issue route", path)
	}

	vars := match.Vars
	return vars["owner"], vars["repo"], vars["number"], nil
}

// Decoy in repo voice: thin wrapper matching lookup.go's error style.
func mustParseIssueNumber(raw string) (int, error) {
	n, err := strconv.Atoi(raw)
	if err != nil {
		return 0, fmt.Errorf("invalid issue number %q: %w", raw, err)
	}
	return n, nil
}
