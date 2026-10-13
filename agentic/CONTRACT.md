# CONTRACT — SearXNG-compatible search API (frozen)

Source of truth: stock SearXNG (`searx/webapp.py::search`,
`searx/webutils.py::get_json_response`, `searx/result_types/_base.py`).
Native must stay byte-compatible at the key level. This file changes only
with a `../native-searxng/PLAN.md` amendment.

## Request

`GET /search` with query params (GET keeps queries out of history trouble;
POST behavior matches stock where applicable):

| Param | Example | Notes |
|---|---|---|
| `q` | `rust async runtime` | required; `400 {"error": ...}` when empty |
| `format` | `json` | `json\|html` in MVP (`csv\|rss` later); stock defaults `formats: [html]` — native enables `json` from day one |
| `pageno` | `1` | 1-based, engines that support paging |
| `language` | `en` | `auto` allowed (browser detection N/A for agents — pass explicit) |
| `safesearch` | `0` | `0\|1\|2` = None/Moderate/Strict |
| `categories` | `general` | comma-separated; unknown → proxied to Python reference |
| `engines` | `duckduckgo,bing` | comma-separated; unknown → proxied to Python reference |

Also served: `GET /` (tiny HTML form), `GET /healthz`, `GET /opensearch.xml`.

## Response (`format=json`, `application/json`)

Top-level keys — identical to stock:

```json
{
  "query": "rust async runtime",
  "results": [ { "url": "...", "title": "...", "content": "...",
                 "engine": "duckduckgo", "engines": ["duckduckgo"],
                 "publishedDate": null, "positions": [1],
                 "score": 0.0, "category": "general", "template": "default.html" } ],
  "answers": [],
  "corrections": [],
  "infoboxes": [],
  "suggestions": [],
  "unresponsive_engines": []
}
```

Result-object rules: `url, title, content` always present (possibly `""`,
never missing); `engine` = first contributing engine, `engines` = full set;
`publishedDate` = ISO string or `null`; media keys (`img_src`, `thumbnail`,
`iframe_src`, `audio_src`) present-but-empty for general results.
Errors: HTTP 400/500 with `{"error": "<message>"}` (mirrors `index_error`).

## Example

```sh
curl -s 'http://127.0.0.1:8888/search?q=rust%20async%20runtime&format=json&pageno=1&language=en' \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); print(len(d["results"]), "results"); print(d["results"][0]["title"])'
```
