VERSION := $(shell cat version.txt)

.PHONY: i dev build bump patch tag clean

i:
	cd frontend && pnpm install

dev:
	cd src-tauri && cargo tauri dev

build:
	@echo "Building PortFinder v$(VERSION)"
	cd src-tauri && cargo tauri build

# bump: set version to today's date (new day release)
bump:
	@NEW_VERSION=$$(date +"%Y.%-m.%-d"); \
	echo "$$NEW_VERSION" > version.txt; \
	sed -i '' 's/^version = ".*"/version = "'$$NEW_VERSION'"/' src-tauri/Cargo.toml; \
	sed -i '' 's/"version": ".*"/"version": "'$$NEW_VERSION'"/' src-tauri/tauri.conf.json; \
	echo "Version bumped to $$NEW_VERSION"

# patch: increment the patch number for today's version
patch:
	@CURRENT=$$(cat version.txt); \
	DATE_PART=$$(echo "$$CURRENT" | cut -d'-' -f1); \
	PATCH_PART=$$(echo "$$CURRENT" | grep -o '\-[0-9]*$$' | tr -d '-'); \
	if [ -z "$$PATCH_PART" ]; then \
		NEW_VERSION="$$DATE_PART-1"; \
	else \
		NEW_PATCH=$$((PATCH_PART + 1)); \
		NEW_VERSION="$$DATE_PART-$$NEW_PATCH"; \
	fi; \
	echo "$$NEW_VERSION" > version.txt; \
	sed -i '' 's/^version = ".*"/version = "'$$NEW_VERSION'"/' src-tauri/Cargo.toml; \
	sed -i '' 's/"version": ".*"/"version": "'$$NEW_VERSION'"/' src-tauri/tauri.conf.json; \
	echo "Version patched to $$NEW_VERSION"

# tag: create and push a git tag from version.txt (triggers release workflow)
tag:
	git tag v$(VERSION)
	git push origin v$(VERSION)

clean:
	cd src-tauri && cargo clean
	rm -rf frontend/dist

.DEFAULT_GOAL := build
