# ID: Library/Homebrew/download_strategy/download_strategy_detector.rb:64
def strategy_for_symbol(symbol)
  strategies = {
    hg:            MercurialDownloadStrategy,
    nounzip:       NoUnzipCurlDownloadStrategy,
    git:           GitDownloadStrategy,
    bzr:           BazaarDownloadStrategy,
    svn:           SubversionDownloadStrategy,
    curl:          CurlDownloadStrategy,
    homebrew_curl: HomebrewCurlDownloadStrategy,
    cvs:           CVSDownloadStrategy,
    post:          CurlPostDownloadStrategy,
    fossil:        FossilDownloadStrategy,
  }

  strategy = strategies[symbol]
  raise TypeError, "Unknown download strategy #{symbol} was requested." if strategy.nil?

  strategy
end
