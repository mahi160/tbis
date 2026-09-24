# 11 — Episode name cleanup

**What to build:** Some Episode names repeat the Series and episode code, such as "The Office (US) - S04E15 - Night Out". Everywhere an Episode name is shown, a leading "Anything - S04E15 - " is removed when its code matches the Episode's own season and episode numbers (including ranges such as S04E18-19), leaving "Night Out". Names without a matching code are never changed.

**Blocked by:** 05 — Series page and Series detail

**Status:** ready-for-agent

- [ ] "The Office (US) - S04E15 - Night Out" shows as "S4E15 · Night Out"
- [ ] "The Office (US) - S04E18-19 - Goodbye, Toby" shows as "S4E18 · Goodbye, Toby"
- [ ] A name whose code does not match the Episode's numbers is left as is
