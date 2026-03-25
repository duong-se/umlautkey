# Project Settings
APP_NAME := UmlautKey
APP_BUNDLE := $(APP_NAME).app
BINARY_NAME := umlautkey
RELEASE_BIN := target/release/$(BINARY_NAME)

# Paths
MACOS_DIR := ./src/platform/macos
PLIST_PATH := $(MACOS_DIR)/Info.plist
ICON_PATH := $(MACOS_DIR)/umlautkey_designed.icns
DMG_NAME := $(APP_NAME)-Installer.dmg
VOL_NAME := $(APP_NAME) Installation

.PHONY: all release-mac clean help dmg

all: release-mac

## release-mac: Build the Rust binary and package it into a macOS .app bundle
release-mac:
	@echo "🔨 Building Rust project in release mode..."
	cargo bundle --release

	@echo "🚀 Build Complete: $(APP_BUNDLE) is ready."
	@echo "ℹ️  Note: You may need to grant Accessibility permissions in System Settings."

## clean: Remove build artifacts and the .app bundle
clean:
	@echo "🧹 Cleaning up..."
	rm -rf $(APP_BUNDLE)
	cargo clean

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -E '^##' Makefile | sed -e 's/## //g' -e 's/: / - /g'


dmg: release-mac
	@echo "💿 Creating DMG installer..."
	@# Remove old DMG if it exists
	rm -f $(DMG_NAME)
	
	create-dmg \
		--volname "$(VOL_NAME)" \
		--volicon "$(ICON_PATH)" \
		--window-pos 200 120 \
		--window-size 600 400 \
		--icon-size 100 \
		--icon "$(APP_BUNDLE)" 175 190 \
		--hide-extension "$(APP_BUNDLE)" \
		--app-drop-link 425 190 \
		"$(DMG_NAME)" \
		"$(APP_BUNDLE)"

	@echo "✅ DMG Created: $(DMG_NAME)"
