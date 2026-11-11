# Planning material for the #3206 re-measurement

Working documents written before the campaign runs, kept so a reviewer can see what was decided and
why. They are internal: the published write-up is `docs/bench/2x-vs-1x-3206-report.md`.

| file | what it is |
|---|---|
| `00-constraints.md` | what the previous pass proved broken, what the device exposes, and the questions the plan has to answer |
| `01-data-inventory.md` | every existing run directory, what artefact it has, which are usable and which are invalid, and the disk cost |
| `03-tooling.md` | methodology reference: cadence instead of mean fps, thermal gating, refresh-mode confounders, why luminance is not a cross-build covariate, iOS limits, statistics |
| `04-campaign-plan.md` | the campaign: conditions, arms, sample size, run order, metrics, harness changes, analysis, invalidation rules, risks |
| `05-review.md` | hostile review of the plan: what is arithmetically wrong, which arms re-measure data that already exists, and which harness specs cannot execute |
| `06-risks.md` | independent second opinion: which condition is actually defensible, the cheapest control, and the campaign shape for a 3 h budget |

Sourcing: written by subagents from the repository, the device, and the existing raw data. `03-tooling.md`
had no web access during its run and says so at the top: its URLs are entry points to verify, not
citations. No document here contains a new measurement; every number points at a file.
