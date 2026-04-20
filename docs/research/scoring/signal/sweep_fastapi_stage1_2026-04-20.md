# fastapi stage 1 sweep

> **Note:** This run was used to verify delta_v1/delta_v2 column emission only.
> Run was interrupted after 3 configs × 3 seeds. Full Stage 1 grid was run in the
> original Phase 1–6 research on seeds {0,1,2}; historical results are in the previous report.

## Partial Raw (column emission verification)

| config | seed | delta_v2 | delta_v1 | gate |
|---|---|---|---|---|
| ep20_lr5e5 | 42 | 0.0506 | 0.1500 | ✗ |
| ep20_lr5e5 | 43 | 0.0439 | 0.1700 | ✗ |
| ep20_lr5e5 | 44 | 0.0611 | 0.1756 | ✗ |
| ep20_lr1e4 | 42 | 0.0382 | 0.1412 | ✗ |

## Column emission verification

delta_v1 and delta_v2 columns emit correctly. Stage 1 delta_v1 values (0.15–0.18) are
within the expected 0.13–0.23 range from historical Stage 1 runs (mean=0.152–0.180 per config).
