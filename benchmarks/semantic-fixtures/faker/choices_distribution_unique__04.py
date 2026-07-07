# ID: faker/utils/distribution.py:26
def weighted_sample_without_replacement(a, p, random=None, length=1):
    # Sample `length` unique items honoring per-item weights, removing each
    # drawn item so it can't be picked twice.
    if random is None:
        random = mod_random

    assert p is not None
    assert len(a) == len(p)
    assert len(a) >= length, "You can't request more unique samples than elements in the dataset."

    picked = []
    remaining_items = list(a)
    remaining_weights = list(p)
    for _ in range(length):
        cdf = tuple(cumsum(remaining_weights))
        total = cdf[-1]
        normalized_cdf = [c / total for c in cdf]
        sample = random_sample(random=random)
        idx = bisect.bisect_right(normalized_cdf, sample)
        picked.append(remaining_items[idx])
        remaining_weights.pop(idx)
        remaining_items.pop(idx)
    return picked
