# 25 — Warn about unencrypted remote servers

**What to build:** A server address typed without a scheme becomes `http://`, so a password and token sent to a public address travel unencrypted. Keep accepting it (many home servers are plain HTTP on port 8096), and show a clear caption on the sign-in screen when the address is `http://` and the host is not on a private network.

**Model:** haiku

**Blocked by:** None.

**Status:** ready-for-agent

**Read first:** `src/jellyfin.rs` (`fn normalize_server` and its tests), `src/login.rs` (`fn render`, where the Server field and error text are placed), `src/components/typography.rs` (`caption`).

## Steps

1. In `src/jellyfin.rs`, add next to `normalize_server`:

   ```rust
   /// True when `server` (already normalized) is plain HTTP to a host outside private
   /// networks: sign-in details would cross the internet unencrypted.
   pub fn is_unencrypted_remote(server: &str) -> bool {
       let Ok(url) = Url::parse(server) else {
           return false;
       };
       if url.scheme() != "http" {
           return false;
       }
       match url.host() {
           Some(url::Host::Ipv4(ip)) => {
               // 100.64.0.0/10 is carrier-grade NAT space, used by Tailscale
               let cgnat = ip.octets()[0] == 100 && (64..128).contains(&ip.octets()[1]);
               !(ip.is_private() || ip.is_loopback() || ip.is_link_local() || cgnat)
           }
           Some(url::Host::Ipv6(ip)) => !(ip.is_loopback() || ip.is_unique_local()),
           Some(url::Host::Domain(name)) => {
               let name = name.to_ascii_lowercase();
               // a bare name (no dot) or .local/.lan/.home is a machine on the local network
               name.contains('.')
                   && name != "localhost"
                   && ![".local", ".lan", ".home", ".internal", ".ts.net"]
                       .iter()
                       .any(|suffix| name.ends_with(suffix))
           }
           None => false,
       }
   }
   ```

   Confirm each `std::net` method exists on the toolchain in use (`cargo check` tells you); replace one that is unstable with an explicit range check.
2. Add a test beside the `normalize_server` tests covering: `http://192.168.1.5:8096` false, `http://100.100.1.2:8096` false, `http://nas:8096` false, `http://jellyfin.local` false, `https://example.com` false, `http://example.com:8096` true, `http://8.8.8.8` true.
3. In `src/login.rs`, `fn render`, under the Server field, add when the check is true for `jellyfin::normalize_server(&self.server.read(cx).value())` and the field is not empty:

   ```rust
   caption("This address is not encrypted. Use https:// if your server supports it.", cx)
   ```

## Done when

- [ ] The caption shows for a public `http://` address and for a public host typed without a scheme
- [ ] It does not show for private, loopback, tailnet, local-name or `https://` addresses
- [ ] Signing in still works in every case
- [ ] The seven test cases pass

**Check:** the gate in `PLAN.md`.
