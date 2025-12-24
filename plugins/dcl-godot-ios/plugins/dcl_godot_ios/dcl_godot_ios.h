#ifndef WEB_KIT_H
#define WEB_KIT_H

#include "core/object/class_db.h"
#include "core/version.h"
#include "core/io/image.h"
#include "core/variant/typed_array.h"

#ifdef __OBJC__
@class ASWebAuthenticationSession;
@class WebKitAuthenticationDelegate;
@class CalendarEventDelegate;
@class UNUserNotificationCenter;
@class NotificationDatabase;
@class AVPlayerWrapper;
#else
typedef void ASWebAuthenticationSession;
typedef void WebKitAuthenticationDelegate;
typedef void CalendarEventDelegate;
typedef void UNUserNotificationCenter;
typedef void NotificationDatabase;
typedef void AVPlayerWrapper;
#endif

#include <map>

class DclGodotiOS : public Object {
    GDCLASS(DclGodotiOS, Object);

    static DclGodotiOS *instance;
    static void _bind_methods();

    ASWebAuthenticationSession *authSession;
    WebKitAuthenticationDelegate *authDelegate;
    CalendarEventDelegate *calendarDelegate;
    NotificationDatabase *notificationDatabase;

    // AVPlayer management
    std::map<int, AVPlayerWrapper*> avPlayers;
    int nextAvPlayerId;

public:
	static String receivedUrl;

    void print_version();
    void open_auth_url(String url);
    void open_webview_url(String url);
    String get_deeplink_url();
    Dictionary get_mobile_device_info();
    Dictionary get_mobile_metrics();
    bool add_calendar_event(String title, String description, int64_t start_time, int64_t end_time, String location);
    bool share_text(String text);
    bool share_text_with_image(String text, Ref<Image> image);

    // Local notifications - Phase 1 API
    void request_notification_permission();
    bool has_notification_permission();
    bool schedule_local_notification(String notification_id, String title, String body, int delay_seconds);
    bool cancel_local_notification(String notification_id);
    bool cancel_all_local_notifications();
    void clear_badge_number();

    // Database API - Phase 3 unified queue management
    bool db_insert_notification(String id, String title, String body, int64_t trigger_timestamp, int is_scheduled, String data, String image_base64);
    bool db_update_notification(String id, Dictionary updates);
    bool db_delete_notification(String id);
    TypedArray<Dictionary> db_query_notifications(String where_clause, String order_by, int limit);
    int db_count_notifications(String where_clause);
    int db_clear_expired(int64_t current_timestamp);
    bool db_mark_scheduled(String id, bool is_scheduled);
    Dictionary db_get_notification(String id);
    int db_clear_all();
    String db_get_notification_image_blob(String id);

    // OS Notification API - Phase 3 renamed for clarity
    bool os_schedule_notification(String notification_id, String title, String body, int delay_seconds);
    bool os_cancel_notification(String notification_id);
    PackedStringArray os_get_scheduled_ids();

    // AVPlayer API - Hardware-accelerated video playback with zero-copy GPU textures
    int createAVPlayer();
    void avPlayerRelease(int player_id);
    int avPlayerInitSurface(int player_id, int width, int height);
    bool avPlayerSetSourceUrl(int player_id, String url);
    bool avPlayerSetSourceLocal(int player_id, String file_path);
    void avPlayerPlay(int player_id);
    void avPlayerPause(int player_id);
    void avPlayerStop(int player_id);
    void avPlayerSetPosition(int player_id, float position_sec);
    float avPlayerGetPosition(int player_id);
    float avPlayerGetDuration(int player_id);
    bool avPlayerIsPlaying(int player_id);
    int avPlayerGetVideoWidth(int player_id);
    int avPlayerGetVideoHeight(int player_id);
    bool avPlayerHasVideoSizeChanged(int player_id);
    int avPlayerGetTextureWidth(int player_id);
    int avPlayerGetTextureHeight(int player_id);
    void avPlayerSetVolume(int player_id, float volume);
    float avPlayerGetVolume(int player_id);
    void avPlayerSetLooping(int player_id, bool loop);
    void avPlayerSetPlaybackRate(int player_id, float rate);
    bool avPlayerHasNewPixelBuffer(int player_id);
    uint64_t avPlayerAcquireIOSurfacePtr(int player_id);
    String avPlayerGetInfo(int player_id);

    // Called from Objective-C when a deeplink is received
    static void emit_deeplink_received(String url);

    static DclGodotiOS *get_singleton();

    DclGodotiOS();
    ~DclGodotiOS();
};

#endif // WEB_KIT_H
