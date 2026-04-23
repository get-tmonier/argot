from pathlib import Path

from argot_bench.extract import ensure_extracted


def test_ensure_extracted_skips_if_output_exists(tmp_path: Path, monkeypatch):
    out = tmp_path / "fastapi" / "SHA" / "dataset.jsonl"
    out.parent.mkdir(parents=True)
    out.write_text("existing\n")

    captured = []
    def fake_run(cmd, **kw):
        captured.append(cmd)

    monkeypatch.setattr("subprocess.run", fake_run)

    result = ensure_extracted(tmp_path / "fastapi" / ".repo", out)
    assert result == out
    assert captured == []  # no subprocess calls


def test_ensure_extracted_runs_argot_extract(tmp_path: Path, monkeypatch):
    repo = tmp_path / "demo" / ".repo"
    repo.mkdir(parents=True)
    out = tmp_path / "demo" / "SHA" / "dataset.jsonl"

    calls = []
    def fake_run(cmd, **kw):
        calls.append(cmd)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text("{}\n")

    monkeypatch.setattr("subprocess.run", fake_run)
    result = ensure_extracted(repo, out)
    assert result == out
    cmdstr = " ".join(calls[0])
    assert "argot-extract" in cmdstr
    assert str(repo) in cmdstr
    assert str(out) in cmdstr
