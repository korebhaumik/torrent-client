#!/usr/bin/env python3
"""Run the real Rust HTTP client against a running Postman simulation."""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
env = dict(os.environ, TORRENT_SEARCH_BASE="http://127.0.0.1:4900/search")

def search(query, source="all"):
    result = subprocess.run(
        [str(root / "target/debug/torrent-desktop"), "--search", query, "--source", source],
        env=env, capture_output=True, text=True, timeout=45, check=True,
    )
    return json.loads(result.stdout)

all_results = search("Ubuntu")
assert not all_results["warnings"], all_results
assert [r["source"] for r in all_results["results"]] == ["piratebay", "tv", "yts", "fitgirl"]
assert all_results["results"][3]["size"] == 1610612736
assert all_results["results"][0]["category"] == "Applications"
for source in ["piratebay", "tv", "yts", "fitgirl"]:
    result = search("Ubuntu", source)
    assert len(result["results"]) == 1 and not result["warnings"], result
    assert result["results"][0]["source"] == source
partial = search("Ubuntu-partial")
assert len(partial["warnings"]) == 1 and len(partial["results"]) == 3, partial
for query in ["outage", "malformed"]:
    result = search(query)
    assert not result["results"] and len(result["warnings"]) == 4, result
assert not search("u")["results"]
print("Search HTTP checks passed: all providers, individual sources, partial outage, HTTP 503, malformed JSON, short query.")
