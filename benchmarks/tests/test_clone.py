from pathlib import Path

from argot_bench.clone import ensure_clone, ensure_sha_checked_out


def test_ensure_clone_uses_existing_repo(tmp_path: Path, monkeypatch):
    # Create a fake existing repo
    repo_dir = tmp_path / "fastapi" / ".repo"
    (repo_dir / ".git").mkdir(parents=True)

    captured = []
    def fake_run(cmd, **kw):
        captured.append(cmd)

    monkeypatch.setattr("subprocess.run", fake_run)

    result = ensure_clone(tmp_path, "fastapi", "https://example.com/fastapi")
    # Should call git fetch, not clone
    assert result == repo_dir
    assert any("fetch" in c for c in captured)
    assert not any("clone" in c for c in captured)


def test_ensure_clone_clones_fresh(tmp_path: Path, monkeypatch):
    captured = []
    def fake_run(cmd, **kw):
        captured.append(cmd)
        # Simulate clone success by creating .git dir
        for i, p in enumerate(cmd):
            if p == "clone":
                target = cmd[-1]
                Path(target, ".git").mkdir(parents=True, exist_ok=True)

    monkeypatch.setattr("subprocess.run", fake_run)

    result = ensure_clone(tmp_path, "fastapi", "https://example.com/fastapi")
    assert any("clone" in c for c in captured)
    assert result == tmp_path / "fastapi" / ".repo"


def test_ensure_sha_checked_out_invokes_git(tmp_path: Path, monkeypatch):
    repo_dir = tmp_path / "demo" / ".repo"
    repo_dir.mkdir(parents=True)
    captured = []

    def fake_run(cmd, **kw):
        captured.append(cmd)

    monkeypatch.setattr("subprocess.run", fake_run)
    ensure_sha_checked_out(repo_dir, "abc123" + "0" * 34)
    cmdstr = " ".join(captured[-1])
    assert "checkout" in cmdstr
    assert "abc123" in cmdstr
