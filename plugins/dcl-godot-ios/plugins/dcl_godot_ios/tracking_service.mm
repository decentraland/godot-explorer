#import "tracking_service.h"

#import <AdSupport/AdSupport.h>
#import <AppTrackingTransparency/AppTrackingTransparency.h>
#import <UIKit/UIKit.h>

static NSString *const kZeroUuid = @"00000000-0000-0000-0000-000000000000";

static NSString *status_name(ATTrackingManagerAuthorizationStatus status) {
	switch (status) {
		case ATTrackingManagerAuthorizationStatusAuthorized:
			return @"authorized";
		case ATTrackingManagerAuthorizationStatusDenied:
			return @"denied";
		case ATTrackingManagerAuthorizationStatusRestricted:
			return @"restricted";
		case ATTrackingManagerAuthorizationStatusNotDetermined:
			return @"not_determined";
	}
	return @"not_determined";
}

NSString *tracking_authorization_status(void) {
	return status_name(ATTrackingManager.trackingAuthorizationStatus);
}

static void request_now(void (^completion)(NSString *)) {
	[ATTrackingManager requestTrackingAuthorizationWithCompletionHandler:^(ATTrackingManagerAuthorizationStatus status) {
		NSString *name = status_name(status);
		dispatch_async(dispatch_get_main_queue(), ^{
			completion(name);
		});
	}];
}

void tracking_request_authorization(void (^completion)(NSString *status)) {
	if (ATTrackingManager.trackingAuthorizationStatus != ATTrackingManagerAuthorizationStatusNotDetermined) {
		// Still async: the caller is usually inside a Godot frame and expects a signal, not a re-entry.
		NSString *name = tracking_authorization_status();
		dispatch_async(dispatch_get_main_queue(), ^{
			completion(name);
		});
		return;
	}
	if (UIApplication.sharedApplication.applicationState == UIApplicationStateActive) {
		request_now(completion);
		return;
	}
	__block id observer = [[NSNotificationCenter defaultCenter]
			addObserverForName:UIApplicationDidBecomeActiveNotification
						object:nil
						 queue:[NSOperationQueue mainQueue]
					usingBlock:^(NSNotification *note) {
						[[NSNotificationCenter defaultCenter] removeObserver:observer];
						observer = nil;
						request_now(completion);
					}];
}

NSString *tracking_advertising_id(void) {
	if (ATTrackingManager.trackingAuthorizationStatus != ATTrackingManagerAuthorizationStatusAuthorized) {
		return nil;
	}
	NSString *idfa = ASIdentifierManager.sharedManager.advertisingIdentifier.UUIDString;
	if (idfa.length == 0 || [idfa isEqualToString:kZeroUuid]) {
		return nil;
	}
	return idfa;
}

NSString *tracking_vendor_id(void) {
	NSString *idfv = UIDevice.currentDevice.identifierForVendor.UUIDString;
	if (idfv.length == 0 || [idfv isEqualToString:kZeroUuid]) {
		return nil;
	}
	return idfv;
}
