# ID: Library/Homebrew/utils/git.rb:192
def apply_cherry_pick(repo, *args, resolve: false, verbose: false)
  cmd = [git.to_s, "-C", repo, "cherry-pick", *args]
  output = Utils.popen_read(*cmd, err: :out)

  unless $CHILD_STATUS.success?
    system git.to_s, "-C", repo.to_s, "cherry-pick", "--abort" unless resolve
    raise ErrorDuringExecution.new(cmd, status: $CHILD_STATUS, output: [[:stdout, output]])
  end

  puts output if verbose
  output
end
