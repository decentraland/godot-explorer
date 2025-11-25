#ifndef WEB_KIT_H
#define WEB_KIT_H

#include "core/object/class_db.h"
#include "core/version.h"
#include "core/io/image.h"

#ifdef __OBJC__
@class ASWebAuthenticationSession;
@class WebKitAuthenticationDelegate;
@class CalendarEventDelegate;
#else
typedef void ASWebAuthenticationSession;
typedef void WebKitAuthenticationDelegate;
typedef void CalendarEventDelegate;
#endif

class DclGodotiOS : public Object {
    GDCLASS(DclGodotiOS, Object);

    static DclGodotiOS *instance;
    static void _bind_methods();

    ASWebAuthenticationSession *authSession;
    WebKitAuthenticationDelegate *authDelegate;
    CalendarEventDelegate *calendarDelegate;

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

    // Called from Objective-C when a deeplink is received
    static void emit_deeplink_received(String url);

    static DclGodotiOS *get_singleton();

    DclGodotiOS();
    ~DclGodotiOS();
};

#endif // WEB_KIT_H
