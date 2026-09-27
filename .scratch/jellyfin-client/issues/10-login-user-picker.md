# 10 — Login user picker

**What to build:** Once the server address is typed and the server answers, Login lists the server's public users by name. Clicking a user without a password signs in right away. Clicking a user with a password fills Username and moves the cursor to Password. Servers that hide their users show no list, and the form works as before.

**Blocked by:** 01 — App shell, Jellyfin login, and saved session

**Status:** done

- [x] Public users appear after the server address is typed
- [x] A passwordless user signs in with one click
- [x] A user with a password fills Username and focuses Password
- [x] An unreachable server or an empty user list shows no picker and no error until Sign in
