// No-op GDExtension stub for platforms where the real Swift framework is not
// available (macOS / Linux / Windows desktop builds). Exports the
// `dcl_swift_lib_init` entry symbol so Godot loads the library cleanly without
// the "No GDExtension library found" error. Registers no classes, so
// `ClassDB.class_exists("DclStoreKit")` still returns false on these platforms
// and `iap_manager.gd` correctly falls back to its disabled path.

#include <stddef.h>
#include <stdint.h>

typedef uint8_t GDExtensionBool;
typedef void *GDExtensionClassLibraryPtr;
typedef void *(*GDExtensionInterfaceGetProcAddress)(const char *p_name);

typedef enum {
	GDEXTENSION_INITIALIZATION_CORE = 0,
	GDEXTENSION_INITIALIZATION_SERVERS = 1,
	GDEXTENSION_INITIALIZATION_SCENE = 2,
	GDEXTENSION_INITIALIZATION_EDITOR = 3,
} GDExtensionInitializationLevel;

typedef struct {
	GDExtensionInitializationLevel minimum_initialization_level;
	void *userdata;
	void (*initialize)(void *userdata, GDExtensionInitializationLevel p_level);
	void (*deinitialize)(void *userdata, GDExtensionInitializationLevel p_level);
} GDExtensionInitialization;

static void stub_initialize(void *userdata, GDExtensionInitializationLevel level) {
	(void)userdata;
	(void)level;
}

static void stub_deinitialize(void *userdata, GDExtensionInitializationLevel level) {
	(void)userdata;
	(void)level;
}

__attribute__((visibility("default")))
GDExtensionBool dcl_swift_lib_init(
		GDExtensionInterfaceGetProcAddress p_get_proc_address,
		GDExtensionClassLibraryPtr p_library,
		GDExtensionInitialization *r_initialization) {
	(void)p_get_proc_address;
	(void)p_library;
	if (r_initialization != NULL) {
		r_initialization->minimum_initialization_level = GDEXTENSION_INITIALIZATION_SCENE;
		r_initialization->userdata = NULL;
		r_initialization->initialize = stub_initialize;
		r_initialization->deinitialize = stub_deinitialize;
	}
	return 1;
}
