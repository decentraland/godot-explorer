//
// Firebase Analytics bridge for the DclGodotiOS plugin.
//

#ifndef dcl_godot_ios_firebase_service_h
#define dcl_godot_ios_firebase_service_h

#ifdef __cplusplus
extern "C" {
#endif

// Configures FirebaseApp once, at plugin init, when the SDK is linked and
// GoogleService-Info.plist is in the app bundle. Otherwise Firebase stays unavailable and
// every call below is a no-op. Called from register_dcl_godot_ios_types.
void force_firebase_service_initialization();

bool firebase_is_available();

#ifdef __cplusplus
}
#endif

#ifdef __OBJC__
#import <Foundation/Foundation.h>

// Empty until Analytics has produced the id (it is nil for the first moments after configure).
NSString *firebase_app_instance_id();

// Empty `value` clears the property / user id, like the Android bridge.
bool firebase_set_user_id(NSString *user_id);
bool firebase_set_user_property(NSString *name, NSString *value);

// `params_json` is a JSON object of string / number / boolean values (or empty).
bool firebase_log_event(NSString *event_name, NSString *params_json);
#endif

#endif /* dcl_godot_ios_firebase_service_h */
