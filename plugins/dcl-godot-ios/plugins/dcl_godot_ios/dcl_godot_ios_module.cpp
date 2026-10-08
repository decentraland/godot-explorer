#include "dcl_godot_ios_module.h"

#include "core/config/engine.h"

#include "dcl_godot_ios.h"
#include "deeplink_service.h"
#include "notification_service.h"
#include "push_service.h"

DclGodotiOS *dcl_godot_ios;

void register_dcl_godot_ios_types() {
	// Force the DeeplinkService to initialize and register with Godot's app delegate
	force_deeplink_service_initialization();

	// Force the NotificationService to initialize and set as UNUserNotificationCenter delegate
	force_notification_service_initialization();

	// Inject the APNs delegate callbacks and request this launch's device token
	force_push_service_initialization();

	dcl_godot_ios = memnew(DclGodotiOS);
	Engine::get_singleton()->add_singleton(Engine::Singleton("DclGodotiOS", dcl_godot_ios));
}

void unregister_dcl_godot_ios_types() {
	if (dcl_godot_ios) {
		memdelete(dcl_godot_ios);
	}
}
