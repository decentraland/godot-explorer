#include "dcl_godot_ios_module.h"

#include "core/config/engine.h"

#include "dcl_godot_ios.h"

DclGodotiOS *dcl_godot_ios;

void register_dcl_godot_ios_types() {
	dcl_godot_ios = memnew(DclGodotiOS);
	Engine::get_singleton()->add_singleton(Engine::Singleton("DclGodotiOS", dcl_godot_ios));
}

void unregister_dcl_godot_ios_types() {
	if (dcl_godot_ios) {
		memdelete(dcl_godot_ios);
	}
}
