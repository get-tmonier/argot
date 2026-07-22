# Launch-film claim and asset inventory

**Issue:** EV-06
**Evidence date:** 2026-07-22
**Repository revision inspected:** `98ef01c33f4193715a43da92794c083005284297`

## Result

The remote media is reachable, but the required complete claim transcript,
synchronized captions, source-owner/license record, committed media provenance,
and accessibility receipts are absent. EV-06 is therefore evaluated but does
not satisfy its acceptance criteria. Under the approved [DR-11 conditional
policy (#163)](https://github.com/get-tmonier/argot/issues/163), unavailable
evidence selects removal from the launch path. That is a decision-policy
outcome, not evidence that removal has shipped.

## Repository and remote asset receipts

| Item | Receipt | Observed state |
| --- | --- | --- |
| Modal component | [`Film.astro`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/landing/src/components/Film.astro) | A click/deep-link (`#film`) opens a modal and lazily assigns the remote MP4 and local poster. It supplies a close button, Escape handling, focus restoration, and reduced-motion CSS; it does not implement focus containment or an inert background. |
| Remote MP4 | `https://media.argot.tmonier.com/argot-film-final.mp4` | HTTP 200 on 2026-07-22; 31,535,985 bytes; H.264 video (1080×1350), AAC audio, 45.375 seconds. Downloaded checksum: `sha256:1e522a196b342da211c2ea6592199e0cb5281df49513905addfad6c4bd3e3281`. The URL is mutable remote hosting, not a committed immutable artifact. |
| Local poster | [`argot-film-poster.jpg`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/landing/public/argot-film-poster.jpg) | Committed 720×900 JPEG; `sha256:2278f766a6de960935f5a643ef405af929f0b6eb8a111cc6abccf1fecfbe11ae`. Its JPEG comment identifies `Lavc62.28.102`, consistent with a video-frame export, but this is not an ownership/license record. |
| Repository references | [`README.md`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/README.md) and `Film.astro` | README calls it a “45-second launch film”; the hero control is “Watch the film.” No transcript, VTT/SRT captions, or source/provenance file exists under `landing/public/`. |

## Claim transcript and accessibility inventory

| Surface | Inventory result | Gate status |
| --- | --- | --- |
| Spoken/on-screen film claims | **Not available as a complete transcript.** The asset contains an AAC track, but no committed or remote transcript/caption receipt was found. A poster-frame visual review confirms “your code, still yours.” and “every pattern you set – kept.”; the captured subtitle reads “So your code stays yours. And safe.” These observed fragments are not a complete transcript. | Fails DR-11 transcript and claim-audit requirements. |
| Poster claims | The visual poster uses a shield/check mark and the “your code, still yours.” / “every pattern you set – kept.” wording; the frame subtitle adds “safe.” The safety framing cannot be treated as an approved product claim without CL-01 review. | Fails current claim qualification pending CL-01; retain-and-demote cannot pass. |
| Captions | No `.vtt`, `.srt`, `<track>`, or caption URL is referenced in the component or committed public assets. | Fails. |
| Keyboard/modal behavior | Open controls, close button, Escape, and focus restoration are implemented; the background remains interactive in the DOM and focus can escape the modal. | Fails full focus-containment/inert requirement. |
| Mobile/reduced motion | Component uses a 92vw/30rem stage and a mobile close-button adjustment; reduced-motion removes animations. No rendered target-device accessibility receipt is committed. | Partial implementation, not a pass receipt. |
| Ownership/license/source provenance | No source project, creator/owner, license, editing source, release version, or immutable media URL is committed. | Fails. |

## Reproduction

```sh
curl -L --fail -o /tmp/argot-film-final.mp4 https://media.argot.tmonier.com/argot-film-final.mp4
shasum -a 256 /tmp/argot-film-final.mp4
ffprobe -v error -show_entries format=duration:stream=index,codec_type,codec_name,width,height \
  -of default=noprint_wrappers=1 /tmp/argot-film-final.mp4
shasum -a 256 landing/public/argot-film-poster.jpg
rg -n -i 'track|caption|transcript|argot-film' landing/src landing/public README.md
```

The first three commands produced the remote checksum and 45.375-second media
metadata above. The final search found no caption or transcript artifact.

## DR-11 policy outcome

The retain-and-demote gate fails: the transcript, captions, provenance, claim
qualification, and accessibility requirements are not satisfied. The approved
[DR-11 policy (#163)](https://github.com/get-tmonier/argot/issues/163) defaults
unavailable evidence to **removal from the launch path**. EV-06 remains an
evaluated, acceptance-not-satisfied evidence record; it does not authorize or
evidence an implemented landing, asset, or public-copy change.
