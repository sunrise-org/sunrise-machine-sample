# Working on a native Sunrise mod

Read README.md and compatibility.json before editing. Keep this repository
separate from Sunrise and horizon-mixins. Only export committed pinned Sunrise
sources into ignored target trees; never modify the original checkout or player
resources. Do not commit binaries, exported sources or Minecraft assets.

The runtime crate lives at the project root in src/. The sole mod manifest is
sunrise-mod.toml; the launcher generates the game entry. Keep helper functions
small and gameplay logic testable there. Add behavior through typed Hooks and
Parts. Native function replacement (`[native.*]` sub-tables) is the unstable
route, built for the exact game build with no silent fallback; this sample
declares none, so do not add one without validating it against the pinned game.

Validate with cargo fmt, clippy with warnings denied, cargo test and
the standalone tests. For bundle/compatibility changes also run
python3 tools/test_stage.py and python3 tools/sunrise.py check/test against the
pinned source export. Keep validation claims and supported revisions honest.
