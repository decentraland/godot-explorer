#!/usr/bin/env ruby
# VIBE-CODED SPIKE: wires the Godot host sources, link flags and bundle resources into
# DclRn.xcodeproj. Run from rn/ios after scripts/sync-godot-ios.sh and before `pod install`.
# Idempotent.
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
  next if group.files.any? { |f| f.path == name }

  ref = group.new_reference(name)
  target.source_build_phase.add_file_reference(ref) if compile
end

godot_group = project.main_group['Godot'] || project.main_group.new_group('Godot', 'Godot')

pck = godot_group.files.find { |f| f.path == 'DclRn.pck' } || godot_group.new_reference('DclRn.pck')
target.resources_build_phase.add_file_reference(pck, true) unless target.resources_build_phase.include?(pck)

if Dir.exist?(File.join(root, 'Godot', 'dylibs')) && godot_group.children.none? { |c| c.path == 'dylibs' }
  dylibs = godot_group.new_reference('dylibs')
  dylibs.last_known_file_type = 'folder'
  target.resources_build_phase.add_file_reference(dylibs, true)
end

embed = target.copy_files_build_phases.find { |p| p.name == 'Embed Godot Frameworks' } ||
        target.new_copy_files_build_phase('Embed Godot Frameworks')
embed.symbol_dst_subfolder_spec = :frameworks
Dir.glob(File.join(root, 'Godot', 'Frameworks', '*.framework')).sort.each do |path|
  name = File.basename(path)
  next if godot_group.files.any? { |f| f.path == "Frameworks/#{name}" }

  ref = godot_group.new_reference("Frameworks/#{name}")
  target.frameworks_build_phase.add_file_reference(ref, true)
  build_file = embed.add_file_reference(ref, true)
  build_file.settings = { 'ATTRIBUTES' => %w[CodeSignOnCopy RemoveHeadersOnCopy] }
end

target.build_configurations.each do |config|
  settings = config.build_settings
  settings['SWIFT_OBJC_BRIDGING_HEADER'] = 'DclRn/DclRn-Bridging-Header.h'
  settings['OTHER_LDFLAGS'] = ['$(inherited)', '-ObjC', '-force_load', '$(SRCROOT)/Godot/libgodot.a', '-lc++']
  settings['CLANG_ENABLE_MODULES'] = 'YES'
end

project.save
puts 'DclRn.xcodeproj configured for Godot'
