#ifndef WEB_KIT_H
#define WEB_KIT_H

#include "core/object/class_db.h"
#include "core/version.h"

#ifdef __OBJC__
@class ASWebAuthenticationSession;
@class WebKitAuthenticationDelegate;
#else
typedef void ASWebAuthenticationSession;
typedef void WebKitAuthenticationDelegate;
#endif

class DclGodotiOS : public Object {
    GDCLASS(DclGodotiOS, Object);

    static DclGodotiOS *instance;
    static void _bind_methods();

    ASWebAuthenticationSession *authSession;
    WebKitAuthenticationDelegate *authDelegate;

public:
    void print_version();
    void open_auth_url(String url);
    void open_webview_url(String url);

    static DclGodotiOS *get_singleton();

    DclGodotiOS();
    ~DclGodotiOS();
};

#endif // WEB_KIT_H
