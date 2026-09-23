# 01 — App shell, Login, and session

**What to build:** When the app launches without a saved session, it shows Login with server URL, username, and password fields. If the URL has no scheme, `http://` is prepended, and any trailing `/` is trimmed. The server is checked through its public system info before authenticating. An unreachable server and wrong credentials each get a clear inline error. After a successful login, the session (server URL, user, access token) is saved to a config file that only the user can read, and relaunching goes straight to Home. The app uses a dark theme and a custom title bar with Home, Movies, and Series tabs (empty screens for now) and a user menu that contains Log out. Log out clears the saved session and returns to Login.

**Blocked by:** None. It can start immediately.

**Status:** done

- [ ] `cargo run` on Apple Silicon opens a single dark window with gpui-kit and links brew libmpv (ADR-0002)
- [ ] `192.168.1.5:8096/` is normalized to `http://192.168.1.5:8096`
- [ ] An unreachable server and bad credentials show distinct inline errors
- [ ] A user without a password signs in with the password field left empty
- [ ] The session file is created with 0600 permissions, and relaunching skips Login
- [ ] Title-bar tabs switch between the Home, Movies, and Series screens
- [ ] Log out deletes the session and shows Login
