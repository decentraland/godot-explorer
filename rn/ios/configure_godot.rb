#!/usr/bin/env ruby
# VIBE-CODED SPIKE: wires the Godot host sources, link flags and bundle resources into
# DclRn.xcodeproj, mirroring what Godot's own iOS export project links. Run from rn/ios after
# scripts/sync-godot-ios.sh and before `pod install`. Idempotent.
require 'xcodeproj'

root = __dir__
project = Xcodeproj::Project.open(File.join(root, 'DclRn.xcodeproj'))
target = project.targets.find { |t| t.name == 'DclRn' } or abort('target DclRn not found')
group = project.main_group['DclRn']

{
  'GodotHost.mm' => true,
  'GodotBridge.mm' => true,
  'GodotHost.h' => false,
  'GodotBridge.h' => false,
  'DclRn-Bridging-Header.h' => false
}.each do |name, compile|
  # The DclRn group has no path of its own; its files are referenced as DclRn/<name>.
  path = "DclRn/#{name}"
  next if group.files.any? { |f| f.path == path }

  ref = group.new_reference(path)
  target.source_build_phase.add_file_reference(ref) if compile
end

godot_group = project.main_group['Godot'] || project.main_group.new_group('Godot', 'Godot')
find_or_add = lambda do |path|
  godot_group.files.find { |f| f.path == path } || godot_group.new_reference(path)
end

plugins = find_or_add.call('godot_plugins.cpp')
target.source_build_phase.add_file_reference(plugins, true)

%w[DclRn.pck override.cfg].each do |name|
  target.resources_build_phase.add_file_reference(find_or_add.call(name), true)
end

embed = target.copy_files_build_phases.find { |p| p.name == 'Embed Godot Frameworks' } ||
        target.new_copy_files_build_phase('Embed Godot Frameworks')
embed.symbol_dst_subfolder_spec = :frameworks

# Static libraries: linked only. Dynamic frameworks: linked and embedded.
static = %w[XCFrameworks/dcl_godot_ios.xcframework]
dynamic = %w[
  Frameworks/libdclgodot.framework
  Frameworks/DclSwiftLib.framework
  XCFrameworks/libsentry.ios.release.xcframework
  XCFrameworks/Sentry.xcframework
]
(static + dynamic).each do |path|
  ref = find_or_add.call(path)
  target.frameworks_build_phase.add_file_reference(ref, true)
  next unless dynamic.include?(path)
  next if embed.files_references.include?(ref)

  build_file = embed.add_file_reference(ref, true)
  build_file.settings = { 'ATTRIBUTES' => %w[CodeSignOnCopy RemoveHeadersOnCopy] }
end

# System frameworks the DclGodotiOS plugin needs (same list as the exported project).
%w[SafariServices AuthenticationServices Network EventKit EventKitUI DeviceCheck Photos PhotosUI
   StoreKit GameController CoreHaptics CoreMotion].each do |name|
  next if target.frameworks_build_phase.files_references.any? { |f| f.path&.end_with?("#{name}.framework") }

  target.add_system_framework(name)
end

target.build_configurations.each do |config|
  settings = config.build_settings
  settings['SWIFT_OBJC_BRIDGING_HEADER'] = 'DclRn/DclRn-Bridging-Header.h'
  settings['OTHER_LDFLAGS'] = [
    '$(inherited)', '-ObjC', '-lc++',
    '-force_load', '$(SRCROOT)/Godot/libgodot.a',
    '$(SRCROOT)/Godot/libMoltenVK.a',
    '-Wl,-U,_sentry_gdextension_init'
  ]
  settings['FRAMEWORK_SEARCH_PATHS'] = ['$(inherited)', '$(SRCROOT)/Godot/Frameworks']
  settings['LD_RUNPATH_SEARCH_PATHS'] = ['$(inherited)', '@executable_path/Frameworks']
  settings['CLANG_ENABLE_MODULES'] = 'YES'
  # The engine and its frameworks are arm64 device builds only.
  settings['IPHONEOS_DEPLOYMENT_TARGET'] = '18.0'
end

project.save
puts 'DclRn.xcodeproj configured for Godot'
