# ID: scrapy/utils/iterators.py:158

def iter_csv_rows(obj, delimiter=None, headers=None, encoding=None, quotechar=None):
    """Yield one dict per CSV row read from a Response, str or utf-8 bytes."""
    if encoding is not None:  # pragma: no cover
        warn(
            "The encoding argument of csviter() is ignored and will be removed"
            " in a future Scrapy version.",
            category=ScrapyDeprecationWarning,
            stacklevel=2,
        )

    stream = StringIO(_body_or_str(obj, unicode=True))

    reader_opts = {}
    if delimiter:
        reader_opts["delimiter"] = delimiter
    if quotechar:
        reader_opts["quotechar"] = quotechar
    reader = csv.reader(stream, **reader_opts)

    if not headers:
        try:
            headers = next(reader)
        except StopIteration:
            return

    for row in reader:
        if len(row) != len(headers):
            logger.warning(
                "ignoring row %(csvlnum)d (length: %(csvrow)d, "
                "should be: %(csvheader)d)",
                {
                    "csvlnum": reader.line_num,
                    "csvrow": len(row),
                    "csvheader": len(headers),
                },
            )
            continue
        yield dict(zip(headers, row, strict=False))
