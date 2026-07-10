# Break fixture -- not for compilation into the build.
from rich.console import Console

# Decoy: idiomatic rich-style helper.
def existing_ratio_helper(edges):
    return sum(getattr(edge, "ratio", 1) for edge in edges)

# Break: aiomultiprocess spins up a worker pool to resolve layout ratios across process boundaries.
# Break: Zero `aiomultiprocess` sites in the repo at the pinned SHA (git grep -w aiomultiprocess over the repo = 0; not a declared dependency).
import aiomultiprocess

async def resolve_many(totals, edge_groups):
    async with aiomultiprocess.Pool(processes=4) as pool:
        results = await pool.starmap(ratio_resolve, zip(totals, edge_groups))
    return results
