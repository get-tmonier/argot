"""Per-hunk evidence layer — orientation context for ``argot check`` hits.

See ``.scratch/check-evidence-layer/PRD.md`` for the design rationale.

Each above-threshold hit carries an ``Evidence`` payload that names what
triggered the flag (the offending tokens / callees / imports) and what
typical code in the same dimension looks like in this repo. Per-reason
formatters render the payload to one or two compact lines under the
headline before the hunk body.
"""
