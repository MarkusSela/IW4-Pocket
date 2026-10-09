# IW4 Pocket 0.3.2 (test build, not a public release)

Everything from 0.3.1 (heap per asset kind, graphics errors in the log, new icon) plus the
work of **treuenten** and **neptune**, imported commit by commit with their authorship kept.

**Frame timing** (treuenten)
- Two frames in flight on Mid/High phones, no antialiasing on menu cameras. The Low tier (about 3 GB granted) keeps one frame, because the extra drawable has not been measured on that memory; `IW4L_FRAME_LATENCY=2` in `iw4l-env.txt` tries it.
- A 120 Hz display is asked for its full refresh during matches (`IW4L_PROMOTION=0` turns it off).
- The screen stays awake while a map loads or a match runs.
- `frame pacing:` statistics in the log every 128 frames (`IW4L_FRAME_STATS=0` turns them off); optional present pacing (`IW4L_FRAME_PACE=30|40|60`).
- Pipelined rendering on iOS stays off. `IW4L_PIPELINED_RENDERING=1` is experimental.

**Engine fixes** (treuenten, one is by neptune)
- Controller: Predator steering, killstreak map cursor.
- Menus: Camera view row, text paging, wide-screen background, End Game and lobby return.
- World: steep slopes, portal culling, plants, dropped weapons, riot shields, projectiles, glass natives (`mp_strike` loads), lighting slots, sky model, sound slots, spark fountains.
- Bots: the navigation cache no longer rebuilds when weapon tables change (12-29 s saved per map).

Held back by the author: the floating-bodies fix.

Please send `iw4l-boot.log` and `iw4l-memory-settings.txt` after trying `mp_rust`. Look for `RENDER ERROR`, `heap net growth` and `frame pacing:`.
