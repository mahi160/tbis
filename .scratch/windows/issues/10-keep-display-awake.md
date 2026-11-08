# 10 — Keep the display awake while playing

**What to build:** `platform::windows::power::Awake`: `SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED)` while alive, `ES_CONTINUOUS` on drop. The call is per thread, so make it on a thread that outlives the `Awake` (the main thread) or use `PowerCreateRequest` / `PowerSetRequest`.

**Blocked by:** 06 (needs playback to test).

**Status:** needs-human (code landed; manual checks pending)

- [ ] The screen doesn't dim or sleep during a long playback
- [ ] `powercfg /requests` lists tbis while playing and not after
