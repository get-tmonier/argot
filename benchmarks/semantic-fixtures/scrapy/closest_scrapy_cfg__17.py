# ID: scrapy/utils/conf.py:73

def find_nearest_scrapy_cfg(path=".", prevpath=None):
    """Walk up from *path* until a scrapy.cfg is found; return its path or ''."""
    if prevpath is not None and str(path) == str(prevpath):
        return ""
    path = Path(path).resolve()
    candidate = path / "scrapy.cfg"
    if candidate.exists():
        return str(candidate)
    return find_nearest_scrapy_cfg(path.parent, path)
