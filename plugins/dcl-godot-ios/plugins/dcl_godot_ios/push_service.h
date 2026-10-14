//
// Remote push (APNs) registration for the DclGodotiOS plugin.
//

#ifndef dcl_godot_ios_push_service_h
#define dcl_godot_ios_push_service_h

#ifdef __cplusplus
extern "C" {
#endif

// Makes sure the APNs callbacks are on GDTApplicationDelegate and asks UIKit for
// this launch's device token. The token (or "" on failure) reaches Godot through
// DclGodotiOS::emit_apns_token_ready. Called from register_dcl_godot_ios_types.
void force_push_service_initialization();

#ifdef __cplusplus
}
#endif

#endif /* dcl_godot_ios_push_service_h */
