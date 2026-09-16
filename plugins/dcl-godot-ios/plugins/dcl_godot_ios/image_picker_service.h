//
// Native photo-gallery picker backing ImagePickerService.gd.
//
// Wraps PHPickerViewController (iOS 14+). The picker runs out of process, so
// it needs no NSPhotoLibrary authorization prompt and the app only ever sees
// the single asset the user chose.
//
// The picked image is downscaled and re-encoded as JPEG on a background queue
// before crossing the bridge, so GDScript never holds a full-resolution photo.

#ifndef dcl_godot_ios_image_picker_service_h
#define dcl_godot_ios_image_picker_service_h

// Presents the gallery picker. Exactly one image_picked signal is emitted per
// call, on the main thread: on success with the JPEG bytes and an empty error,
// on user dismissal with an empty buffer and "cancelled", and with a message
// on failure.
//
// `max_dimension` caps the longest edge in pixels (no upscaling).
// `jpeg_quality` is 0..1.
void dcl_present_image_picker(int max_dimension, float jpeg_quality);

#endif /* dcl_godot_ios_image_picker_service_h */
