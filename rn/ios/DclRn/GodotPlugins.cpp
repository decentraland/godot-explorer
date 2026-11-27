// The plugin registration hooks Godot's iOS exporter generates (empty: the app has no Godot
// plugin), copied in by scripts/sync-godot-ios.sh. libgodot.a needs them.
#if DCL_WITH_GODOT
#include "../Godot/godot_plugins.cpp"
#endif
