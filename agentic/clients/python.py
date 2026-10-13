"""Minimal agent integration example. Stdlib only."""
import json
import urllib.parse
import urllib.request

BASE = "http://127.0.0.1:8888"


def search(query, pageno=1, language="en", **extra):
    params = {"q": query, "format": "json",
              "pageno": pageno, "language": language, **extra}
    url = BASE + "/search?" + urllib.parse.urlencode(params)
    with urllib.request.urlopen(url, timeout=30) as r:
        return json.loads(r.read().decode("utf-8"))


if __name__ == "__main__":
    data = search("rust async runtime")
    print(f'query: {data["query"]} ({len(data["results"])} results)')
    for r in data["results"][:5]:
        print(f'- {r["title"]}\n  {r["url"]}')
