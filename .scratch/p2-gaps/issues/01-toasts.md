# 01 — Toasts for transient messages

**What to build:** One consistent toast pattern for short-lived messages (errors that don't block the screen, confirmations like "Marked watched" or "Screenshot saved"). The Player's current one-off PiP-failure banner moves onto it. Prefactor for later tickets that need a transient message.

**Blocked by:** None — can start immediately.

**Status:** done

- [x] Toasts appear briefly, stack if several arrive, and dismiss on their own or on click
- [x] Toasts are readable over the Player's live video as well as over the app background
- [x] PiP start failure uses a toast instead of its bespoke banner
- [x] Uses gpui-kit's notification component if it fits; colors come from the theme
