# 06 — The Movies/Series library loads without freezing the UI

**What to build:** Opening Movies or Series, or changing the sort, fetches the whole Library in one unpaged request, and the JSON is decoded on the main thread, so big libraries freeze the UI. Decode the response off the main thread, or load in pages, so the grid stays responsive while it loads.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified on a large library). Fetch and decode run in a background task; paging not added.

- [ ] Decoding the library response does not run on the main thread
- [ ] On a large library, scrolling and hover stay responsive while Movies/Series loads
- [ ] Sort changes and refresh still replace any in-flight load, and the current list stays visible until fresh data arrives
- [ ] Item order and counts match the current behaviour
