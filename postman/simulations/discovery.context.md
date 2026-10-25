# Torrent discovery simulation

## Purpose

Run the Rust search adapters against the existing development fixtures in
`postman/mocks/search-providers/default.js`. These hand-written examples model the
original provider response shapes; they are not provider-owned contracts.

## Execution

Build the real client, then start the local fixtures:

```sh
./scripts/cargo.sh build --locked
postman simulation run postman/simulations/discovery.sim.yaml --port 4900 --output ndjson
# In another terminal:
python3 scripts/check-search.py
```

Wait for `listening` before running the checks. Redirect NDJSON to an ignored
`.qa/` file when recording a run. Inspect request events by provider path and
status, then stop the simulation with Ctrl-C.

## Assumptions

- Local HTTP listeners and client connections require permission outside the shell
  sandbox. Postman CLI 1.76.0 served the existing simulation without signing in.
  (observed 2026-10-25)
- Passing fixtures establish request routing, normalization, sorting, and failure
  handling, not live provider availability or agreement with future provider data.
  Tracker scraping is skipped when `TORRENT_SEARCH_BASE` is set, so this simulation
  does not verify public UDP trackers. (reasoned 2026-10-25, `native/src/search.rs`)

## Observations

<!-- begin:observations -->
2026-10-25: The optimization regression run served the `search-providers` member
across Pirate Bay, FitGirl REST/RSS, YTS, TVMaze, and EZTV paths; no member stayed
silent. The real Rust client passed all-source and individual searches, partial
outage, HTTP 503, malformed response, and short-query checks.
<!-- end:observations -->
