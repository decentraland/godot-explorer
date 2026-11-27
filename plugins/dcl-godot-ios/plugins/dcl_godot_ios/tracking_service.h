#ifndef TRACKING_SERVICE_H
#define TRACKING_SERVICE_H

#import <Foundation/Foundation.h>

// App Tracking Transparency status: "authorized" | "denied" | "restricted" | "not_determined".
NSString *tracking_authorization_status(void);

// Shows the ATT prompt as soon as the app is active (iOS drops it otherwise). `completion` runs
// on the main queue with the resulting status, without a prompt when the answer is already known.
void tracking_request_authorization(void (^completion)(NSString *status));

// IDFA, or nil unless tracking is authorized and the id is not all zeros.
NSString *tracking_advertising_id(void);

// IDFV, or nil while the device is still locked after a restart.
NSString *tracking_vendor_id(void);

#endif
