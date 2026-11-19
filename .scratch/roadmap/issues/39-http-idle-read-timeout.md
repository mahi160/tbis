# 39 — HTTP requests give up on a stalled server

**What to build:** The HTTP client has a 10 second connect timeout but no read timeout. A server that accepts the connection and then sends nothing holds sign-in, the update check and Player lookups forever. Add a 30 second idle read timeout: it fires only when no bytes arrive for 30 seconds, so large downloads are unaffected.

**Model:** haiku

**Blocked by:** None.

**Status:** implemented; acceptance pending. A 30-second idle read timeout is configured through the shared HTTP client factory. The real stalled-response regression test passed after approximately 30 seconds using the same factory. HTTP and HTTPS sign-in/browsing still require manual acceptance because the constructor also changes TLS configuration. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/main.rs` (`fn main`), and `~/.cargo/registry/src/*/gpui-pre-reqwest-client-0.3.9/src/reqwest_client.rs` (`fn builder`, `pub fn user_agent`, `pub fn proxy_user_agent_and_read_timeout`).

## Steps

1. In `src/main.rs`, above `fn main`, add:

   ```rust
   /// A response that sends nothing for this long is given up on.
   const HTTP_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
   ```

2. In `fn main`, replace:

   ```rust
   let http =
       reqwest_client::ReqwestClient::user_agent(concat!("tbis/", env!("CARGO_PKG_VERSION")))
           .expect("failed to build HTTP client");
   ```

   with:

   ```rust
   let http = reqwest_client::ReqwestClient::proxy_user_agent_and_read_timeout(
       None,
       concat!("tbis/", env!("CARGO_PKG_VERSION")),
       Some(HTTP_IDLE_TIMEOUT),
   )
   .expect("failed to build HTTP client");
   ```

## Known side effect, must be checked by running

`proxy_user_agent_and_read_timeout` also calls `.use_preconfigured_tls(http_client_tls::tls_config())`, which `user_agent` does not. Certificate checking may therefore behave differently. Do not mark this ticket done until someone has signed in to an `https://` server with the new build. If sign-in to a server that worked before now fails with a certificate error, revert the change and write the error under Status.

## Done when

- [ ] Sign-in and browsing work against an `http://` server
- [ ] Sign-in and browsing work against an `https://` server
- [ ] A request to a server that accepts the connection and never answers fails after about 30 seconds (test with `nc -l 8096` as the server address)

**Check:** the gate in `PLAN.md`.
