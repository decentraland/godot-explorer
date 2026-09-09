#include "image_picker_service.h"
#include "dcl_godot_ios.h"
#include "core/os/os.h"
#include "core/string/print_string.h"

#include <string.h>

#import <UIKit/UIKit.h>
#import <PhotosUI/PhotosUI.h>

// Delegate for a single presentation. PHPickerViewController holds only a weak
// reference to its delegate, so the instance keeps itself alive in a static
// slot for the lifetime of the picker and clears it once the result is in.
//
// It is also the presentation-controller delegate: a sheet dismissed by
// swiping down does not reliably deliver didFinishPicking, and without
// presentationControllerDidDismiss: the GDScript awaiter would hang forever.
@interface DclImagePickerDelegate : NSObject <PHPickerViewControllerDelegate,
                                              UIAdaptivePresentationControllerDelegate>
@property (nonatomic, assign) int maxDimension;
@property (nonatomic, assign) float jpegQuality;
@end

// The in-flight delegate, or nil when no picker is on screen. Only ever
// touched on the main thread.
static DclImagePickerDelegate *dcl_active_picker_delegate = nil;

// Guards the one-signal-per-call contract. Several paths can now report the
// same pick (didFinishPicking, swipe-dismiss, an async load completion that
// lands after the sheet is gone), and a second emission would wake a later
// awaiter with a stale result.
static bool dcl_pick_finished = true;

// Emits image_picked once. Must be called on the main thread.
static void dcl_emit_image_picked(NSData *jpeg, NSString *error) {
    PackedByteArray bytes;
    if (jpeg != nil && jpeg.length > 0) {
        bytes.resize(jpeg.length);
        memcpy(bytes.ptrw(), jpeg.bytes, jpeg.length);
    }

    String error_str;
    if (error != nil) {
        error_str = String::utf8([error UTF8String]);
    }

    DclGodotiOS *singleton = DclGodotiOS::get_singleton();
    if (singleton == nullptr) {
        if (OS::get_singleton() != nullptr) {
            print_line("[IMAGE_PICKER] ERROR: singleton unavailable, dropping result");
        }
        return;
    }
    singleton->emit_signal("image_picked", bytes, error_str);
}

// Marshals to the main thread, releases the delegate, then emits.
static void dcl_finish_image_pick(NSData *jpeg, NSString *error) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (dcl_pick_finished) {
            return;
        }
        dcl_pick_finished = true;
        dcl_active_picker_delegate = nil;
        dcl_emit_image_picked(jpeg, error);
    });
}

// Scales `image` so its longest edge is at most `maxDimension`, honouring the
// orientation baked into the UIImage (drawInRect applies it, which is what
// keeps landscape photos upright). Images already small enough are redrawn
// unscaled so the orientation is still normalized.
static UIImage *dcl_downscale_image(UIImage *image, int maxDimension) {
    CGSize size = image.size;
    if (size.width <= 0 || size.height <= 0) {
        return nil;
    }

    CGFloat longest = MAX(size.width, size.height);
    CGFloat scale = 1.0;
    if (maxDimension > 0 && longest > (CGFloat)maxDimension) {
        scale = (CGFloat)maxDimension / longest;
    }

    CGSize target = CGSizeMake(round(size.width * scale), round(size.height * scale));

    UIGraphicsImageRendererFormat *format = [UIGraphicsImageRendererFormat defaultFormat];
    // The source is already in pixels; a device scale factor would silently
    // multiply the output resolution back up past maxDimension.
    format.scale = 1.0;
    format.opaque = YES;

    UIGraphicsImageRenderer *renderer =
        [[UIGraphicsImageRenderer alloc] initWithSize:target format:format];
    return [renderer imageWithActions:^(UIGraphicsImageRendererContext *context) {
        [image drawInRect:CGRectMake(0, 0, target.width, target.height)];
    }];
}

@implementation DclImagePickerDelegate

- (void)picker:(PHPickerViewController *)picker
    didFinishPicking:(NSArray<PHPickerResult *> *)results {
    [picker dismissViewControllerAnimated:YES completion:nil];

    // An empty result set is how PHPicker reports "user tapped Cancel".
    if (results.count == 0) {
        dcl_finish_image_pick(nil, @"cancelled");
        return;
    }

    NSItemProvider *provider = results.firstObject.itemProvider;
    if (![provider canLoadObjectOfClass:[UIImage class]]) {
        dcl_finish_image_pick(nil, @"unsupported_asset");
        return;
    }

    int maxDimension = self.maxDimension;
    float quality = self.jpegQuality;

    // loadObjectOfClass already calls back off the main thread; the decode and
    // JPEG encode below are the expensive part and stay there.
    [provider loadObjectOfClass:[UIImage class]
              completionHandler:^(__kindof id<NSItemProviderReading> object, NSError *error) {
        if (error != nil || ![object isKindOfClass:[UIImage class]]) {
            dcl_finish_image_pick(nil, @"load_failed");
            return;
        }

        UIImage *scaled = dcl_downscale_image((UIImage *)object, maxDimension);
        if (scaled == nil) {
            dcl_finish_image_pick(nil, @"decode_failed");
            return;
        }

        NSData *jpeg = UIImageJPEGRepresentation(scaled, quality);
        if (jpeg == nil || jpeg.length == 0) {
            dcl_finish_image_pick(nil, @"encode_failed");
            return;
        }

        dcl_finish_image_pick(jpeg, nil);
    }];
}

// Swipe-down dismissal. didFinishPicking: is not guaranteed to fire for an
// interactive dismissal, so without this the awaiting coroutine never wakes
// and the calling UI stays stuck. The emit-once guard makes the overlap with
// didFinishPicking: harmless.
- (void)presentationControllerDidDismiss:(UIPresentationController *)presentationController {
    dcl_finish_image_pick(nil, @"cancelled");
}

@end

void dcl_present_image_picker(int max_dimension, float jpeg_quality) {
    dispatch_async(dispatch_get_main_queue(), ^{
        // GDScript guards against concurrent picks too, but a dropped signal
        // would hang an awaiter forever, so the native side refuses as well.
        if (dcl_active_picker_delegate != nil) {
            // Emitted directly, bypassing the one-shot guard: this reports the
            // *new* call, while the guard belongs to the pick still on screen.
            dcl_emit_image_picked(nil, @"already_picking");
            return;
        }

        UIViewController *rootVC = [UIApplication sharedApplication].keyWindow.rootViewController;
        if (rootVC == nil) {
            dcl_emit_image_picked(nil, @"no_root_view_controller");
            return;
        }

        PHPickerConfiguration *config = [[PHPickerConfiguration alloc] init];
        config.selectionLimit = 1;
        config.filter = [PHPickerFilter imagesFilter];

        DclImagePickerDelegate *delegate = [[DclImagePickerDelegate alloc] init];
        delegate.maxDimension = max_dimension;
        delegate.jpegQuality = jpeg_quality;
        dcl_active_picker_delegate = delegate;

        PHPickerViewController *picker =
            [[PHPickerViewController alloc] initWithConfiguration:config];
        picker.delegate = delegate;
        // Catches swipe-to-dismiss, which the picker delegate alone misses.
        picker.presentationController.delegate = delegate;

        dcl_pick_finished = false;
        [rootVC presentViewController:picker animated:YES completion:nil];
    });
}
