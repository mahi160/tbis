# 11 — Update dialog shows the changelog

**What to build:** The "tbis X is available" dialog shows only the version. Show the newest release's notes inside the dialog, rendered as markdown, plus a link that opens that release's GitHub page. Releases today use GitHub's generated notes, which with direct commits are usually just a compare link, so the release workflow must also write real notes: grouped `feat` / `fix` commit subjects since the previous tag, excluding `chore(release)` commits.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] The release workflow publishes notes listing the `feat` and `fix` commit subjects since the previous tag, grouped by type
- [ ] The update dialog shows the newest release's notes as markdown above Install and Restart, scrollable when long
- [ ] The dialog has a link that opens the release's GitHub page in the browser
- [ ] A release with empty notes still shows the dialog, with just the link
- [ ] The manual "Check for Updates…" path shows the same dialog
