# ID: Library/Homebrew/download_strategy/download_strategy_detector.rb:24
def strategy_for_url(url)
  if url.match?(GitHubPackages::URL_REGEX)
    CurlGitHubPackagesDownloadStrategy
  elsif url.match?(%r{^https?://github\.com/[^/]+/[^/]+\.git$})
    GitHubGitDownloadStrategy
  elsif url.match?(%r{^https?://.+\.git$}) || url.match?(%r{^git://}) ||
        url.match?(%r{^https?://git\.sr\.ht/[^/]+/[^/]+$}) ||
        url.match?(%r{^https?://tangled\.sh/[^/]+/[^/]+$}) || url.match?(%r{^ssh://git})
    GitDownloadStrategy
  elsif url.match?(%r{^https?://www\.apache\.org/dyn/closer\.cgi}) ||
        url.match?(%r{^https?://www\.apache\.org/dyn/closer\.lua})
    CurlApacheMirrorDownloadStrategy
  elsif url.match?(%r{^https?://files\.pythonhosted\.org/packages/})
    PyPIDownloadStrategy
  elsif url.match?(%r{^https?://([A-Za-z0-9\-.]+\.)?googlecode\.com/svn}) || url.match?(%r{^https?://svn\.}) ||
        url.match?(%r{^svn://}) || url.match?(%r{^svn\+http://}) ||
        url.match?(%r{^http://svn\.apache\.org/repos/}) ||
        url.match?(%r{^https?://([A-Za-z0-9\-.]+\.)?sourceforge\.net/svnroot/})
    SubversionDownloadStrategy
  elsif url.match?(%r{^cvs://})
    CVSDownloadStrategy
  elsif url.match?(%r{^hg://}) || url.match?(%r{^https?://([A-Za-z0-9\-.]+\.)?googlecode\.com/hg}) ||
        url.match?(%r{^https?://([A-Za-z0-9\-.]+\.)?sourceforge\.net/hgweb/})
    MercurialDownloadStrategy
  elsif url.match?(%r{^bzr://})
    BazaarDownloadStrategy
  elsif url.match?(%r{^fossil://})
    FossilDownloadStrategy
  else
    CurlDownloadStrategy
  end
end
