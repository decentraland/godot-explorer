.PHONY: build xcframework clean

# Build configuration
CONFIG ?= Release
DERIVED_DATA ?= $(CURDIR)/build
SCHEME ?= DclSwiftLib
FRAMEWORK_NAME ?= DclSwiftLib
XCFRAMEWORK ?= $(CURDIR)/bin/$(FRAMEWORK_NAME).xcframework
XCODEBUILD ?= xcodebuild

# Destination for Godot export (relative to this Makefile)
GODOT_IOS_DIR ?= $(CURDIR)/../../godot/ios/dcl-swift-lib

# Xcodebuild flags to skip plugin/macro validation (required for SwiftGodot)
XCODE_FLAGS := -skipPackagePluginValidation -skipMacroValidation

IOS_DEVICE_FRAMEWORK := $(DERIVED_DATA)/Build/Products/$(CONFIG)-iphoneos/PackageFrameworks/$(FRAMEWORK_NAME).framework
IOS_SIM_FRAMEWORK := $(DERIVED_DATA)/Build/Products/$(CONFIG)-iphonesimulator/PackageFrameworks/$(FRAMEWORK_NAME).framework

# Build for iOS device only (quick build for testing)
build:
	@set -e; \
	$(XCODEBUILD) \
		-scheme '$(SCHEME)' \
		-configuration '$(CONFIG)' \
		-destination 'generic/platform=iOS' \
		-derivedDataPath '$(DERIVED_DATA)' \
		$(XCODE_FLAGS) \
		build

# Build universal xcframework (device + simulator)
xcframework:
	@set -e; \
	mkdir -p bin/; \
	echo "Building for iOS device..."; \
	$(XCODEBUILD) \
		-scheme '$(SCHEME)' \
		-configuration '$(CONFIG)' \
		-destination 'generic/platform=iOS' \
		-derivedDataPath '$(DERIVED_DATA)' \
		$(XCODE_FLAGS) \
		build; \
	echo "Building for iOS Simulator..."; \
	$(XCODEBUILD) \
		-scheme '$(SCHEME)' \
		-configuration '$(CONFIG)' \
		-destination 'generic/platform=iOS Simulator' \
		-derivedDataPath '$(DERIVED_DATA)' \
		$(XCODE_FLAGS) \
		build; \
	echo "Copying resource bundles into frameworks..."; \
	for bundle in $(DERIVED_DATA)/Build/Products/$(CONFIG)-iphoneos/*.bundle; do \
		if [ -d "$$bundle" ]; then \
			cp -R "$$bundle" '$(IOS_DEVICE_FRAMEWORK)/'; \
			echo "  Copied $$(basename $$bundle) to device framework"; \
		fi; \
	done; \
	for bundle in $(DERIVED_DATA)/Build/Products/$(CONFIG)-iphonesimulator/*.bundle; do \
		if [ -d "$$bundle" ]; then \
			cp -R "$$bundle" '$(IOS_SIM_FRAMEWORK)/'; \
			echo "  Copied $$(basename $$bundle) to simulator framework"; \
		fi; \
	done; \
	echo "Creating xcframework..."; \
	rm -rf '$(XCFRAMEWORK)'; \
	$(XCODEBUILD) -create-xcframework \
		-framework '$(IOS_DEVICE_FRAMEWORK)' \
		-framework '$(IOS_SIM_FRAMEWORK)' \
		-output '$(XCFRAMEWORK)'; \
	echo "Created: $(XCFRAMEWORK)"; \
	echo "Copying framework to Godot project..."; \
	mkdir -p '$(GODOT_IOS_DIR)'; \
	rm -rf '$(GODOT_IOS_DIR)/$(FRAMEWORK_NAME).framework'; \
	cp -R '$(XCFRAMEWORK)/ios-arm64/$(FRAMEWORK_NAME).framework' '$(GODOT_IOS_DIR)/'; \
	echo "Installed: $(GODOT_IOS_DIR)/$(FRAMEWORK_NAME).framework"

clean:
	rm -rf build bin .build .swiftpm

