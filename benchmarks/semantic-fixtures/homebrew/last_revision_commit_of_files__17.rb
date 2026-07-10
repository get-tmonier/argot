# ID: Library/Homebrew/utils/git.rb:87
def latest_commit_of_files(repo, files, before_commit: nil)
  range_args = if before_commit.nil?
    ["--skip=1"]
  else
    [before_commit.split("..").first]
  end

  output = Utils.popen_read(
    git, "-C", repo, "log",
    "--pretty=format:%h", "--abbrev=7", "--max-count=1",
    "--diff-filter=d", "--name-only", *range_args, "--", *files
  ).lines.map(&:chomp).reject(&:empty?)

  commit, *paths = output
  [commit, paths]
end
