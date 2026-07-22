# Caught-in-the-Wild evidence inventory

**Issue:** EV-05
**Evidence date:** 2026-07-22
**Repository revision inspected:** `98ef01c33f4193715a43da92794c083005284297`

## Result

No displayed case passes the DR-10 receipt gate and the claimed corpus total
cannot be verified. The inventory therefore yields **zero qualifying public
cases** and **no verified repository count**. This is an evidence result, not
a conclusion that any cited change is defective.

The checked source is
[`landing/src/lib/caught-in-the-wild.ts`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/landing/src/lib/caught-in-the-wild.ts).
It declares `REPO_COUNT = 33`, contains five case narratives, and sets every
`upstreamUrl` to `null`.

## Receipt requirements

DR-10 requires, for each retained case: a public repository and commit, command
and range, Argot version/composition/configuration, raw finding JSON and the
actual finding hash, date, adjudication, a reachable upstream URL,
license/privacy status, and an original-versus-reconstruction label. A corpus
count additionally requires one qualifying receipt for every unique repository.

## Case inventory

| Case | Public commit receipt | Source fields present | Missing required receipt fields | Classification |
| --- | --- | --- | --- | --- |
| dagster | [dagster-io/dagster@ecc8b1c](https://github.com/dagster-io/dagster/commit/ecc8b1c23de5b0f987e3cb783599ea85638836b8), reachable via GitHub API on 2026-07-22 | Repository name, commit SHA, path/line range, rendered evidence sentence, story | upstream URL in source; command/range; Argot version/composition/config; raw JSON; finding hash; date; adjudication; reconstruction label; license/privacy receipt | Unverifiable; does not qualify |
| hono | [honojs/hono@5205e7c](https://github.com/honojs/hono/commit/5205e7c7cfdf9dfc2124244c1123ef4050983fd8), reachable via GitHub API on 2026-07-22 | Repository name, commit SHA, path/line range, rendered evidence sentence, story | Same missing fields as dagster | Unverifiable; does not qualify |
| rich | [Textualize/rich@72b0a9e](https://github.com/Textualize/rich/commit/72b0a9e964a32a9d65a9cf895f7758bb85e0c631), reachable via GitHub API on 2026-07-22 | Repository name, commit SHA, path/line range, rendered evidence sentence, story | Same missing fields as dagster | Unverifiable; does not qualify |
| saleor | [saleor/saleor@e2ebabe](https://github.com/saleor/saleor/commit/e2ebabee9dcfb0cc25535a8dfea9a9fb1ab6b119), reachable via GitHub API on 2026-07-22 | Repository name, commit SHA, path/line range, rendered evidence sentence, story | Same missing fields as dagster; this source claims a reconstruction but commits no reconstruction receipt | Unverifiable; does not qualify |
| faker | [joke2k/faker@a1a1b2a](https://github.com/joke2k/faker/commit/a1a1b2acb417c0f14d80292d6cfbf357041f93ee), reachable via GitHub API on 2026-07-22 | Repository name, commit SHA, path/line range, rendered evidence sentence, story | Same missing fields as dagster | Unverifiable; does not qualify |
| 33-repository corpus | None | Hard-coded `REPO_COUNT = 33` only | Identities and qualifying receipts for the other 28 repositories; all corpus-level run/config/finding/adjudication data | Unverifiable; count must be withheld |

## Reproduction assessment

The five source commit identifiers were confirmed reachable with:

```sh
gh api repos/dagster-io/dagster/commits/ecc8b1c23de5b0f987e3cb783599ea85638836b8 --jq .sha
gh api repos/honojs/hono/commits/5205e7c7cfdf9dfc2124244c1123ef4050983fd8 --jq .sha
gh api repos/Textualize/rich/commits/72b0a9e964a32a9d65a9cf895f7758bb85e0c631 --jq .sha
gh api repos/saleor/saleor/commits/e2ebabee9dcfb0cc25535a8dfea9a9fb1ab6b119 --jq .sha
gh api repos/joke2k/faker/commits/a1a1b2acb417c0f14d80292d6cfbf357041f93ee --jq .sha
```

Each returned the requested full SHA. That establishes only the accessibility
of the upstream commit—not an Argot finding, an adjudication, or the story's
claims. A rerun is not reproducible from the repository because the historical
Argot release/configuration, fit corpus/exclusions, command/range, raw output,
and finding hash are absent. This is an evidence-availability failure, not a
licensing or privacy block.

## DR-10 handoff

Apply the approved fallback: withhold the corpus count and all five wild-case
stories unless complete receipts are later added. Do not display a commit SHA
as a finding hash and do not replace the cases with authored fixtures under a
wild-case label.
