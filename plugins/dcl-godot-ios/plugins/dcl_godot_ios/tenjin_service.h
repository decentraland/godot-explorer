#ifndef TENJIN_SERVICE_H
#define TENJIN_SERVICE_H

#import <Foundation/Foundation.h>

// Tenjin MMP bridge. Every function is a no-op (returns false / nil) when the SDK is not
// linked into the build, and start() must run before the others.
bool tenjin_start(NSString *sdk_key, NSString *user_id);
bool tenjin_event(NSString *name, NSInteger value);
void tenjin_handle_open_url(NSString *url);

// Attribution info once Tenjin answered, nil while pending. The first call starts the fetch.
NSDictionary *tenjin_attribution_if_ready(bool *failed);

#endif
