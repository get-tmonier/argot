#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture_dir=$(mktemp -d "${TMPDIR:-/tmp}/argot-pre-commit.XXXXXX")
trap 'rm -rf "$fixture_dir"' EXIT HUP INT TERM

mkdir -p "$fixture_dir/bin"
cat > "$fixture_dir/bin/argot" <<'EOF'
#!/bin/sh
printf '%s\n' "fake argot case ${ARGOT_TEST_CASE:-clean}"
case "${ARGOT_TEST_CASE:-clean}" in
  clean | warn) exit 0 ;;
  error) exit 1 ;;
  unfitted | command-failure) exit 2 ;;
  *) exit 64 ;;
esac
EOF
chmod +x "$fixture_dir/bin/argot"

if command -v pre-commit >/dev/null 2>&1; then
  hook_repo="$fixture_dir/hook-repo"
  target_repo="$fixture_dir/target-repo"
  mkdir "$hook_repo"
  cp "$repo_root/.pre-commit-hooks.yaml" "$hook_repo/.pre-commit-hooks.yaml"
  git -C "$hook_repo" init -q
  git -C "$hook_repo" config user.email integration@example.invalid
  git -C "$hook_repo" config user.name integration-test
  git -C "$hook_repo" add .
  git -C "$hook_repo" commit -qm hooks
  hook_revision=$(git -C "$hook_repo" rev-parse HEAD)

  mkdir "$target_repo"
  git -C "$target_repo" init -q
  git -C "$target_repo" config user.email integration@example.invalid
  git -C "$target_repo" config user.name integration-test
  printf 'print("fixture")\n' > "$target_repo/fixture.py"
  git -C "$target_repo" add fixture.py
  git -C "$target_repo" commit -qm fixture
  cat > "$target_repo/.pre-commit-config.yaml" <<EOF
repos:
  - repo: $hook_repo
    rev: $hook_revision
    hooks:
      - id: argot-check
      - id: argot-check-gate
EOF
  (cd "$target_repo" && PATH="$fixture_dir/bin:$PATH" ARGOT_TEST_CASE=clean pre-commit run argot-check --all-files)
  (cd "$target_repo" && PATH="$fixture_dir/bin:$PATH" ARGOT_TEST_CASE=warn pre-commit run argot-check --all-files)
  (cd "$target_repo" && PATH="$fixture_dir/bin:$PATH" ARGOT_TEST_CASE=error pre-commit run argot-check --all-files)
  set +e
  (cd "$target_repo" && PATH="$fixture_dir/bin:$PATH" ARGOT_TEST_CASE=unfitted pre-commit run argot-check --all-files)
  unfitted_status=$?
  (cd "$target_repo" && PATH="$fixture_dir/bin:$PATH" ARGOT_TEST_CASE=command-failure pre-commit run argot-check --all-files)
  command_failure_status=$?
  (cd "$target_repo" && PATH="$fixture_dir/bin:$PATH" ARGOT_TEST_CASE=error pre-commit run argot-check-gate --all-files)
  error_gate_status=$?
  set -e
  [ "$unfitted_status" -ne 0 ]
  [ "$command_failure_status" -ne 0 ]
  [ "$error_gate_status" -ne 0 ]
  (cd "$target_repo" && PATH="$fixture_dir/bin:$PATH" ARGOT_TEST_CASE=warn pre-commit run argot-check-gate --all-files)
else
  printf '%s\n' 'pre-commit is not installed; real pre-commit behavior matrix was skipped.' >&2
fi
