VERSION := $(shell cat version.txt)
GOPATH ?= $(shell go env GOPATH)
WAILS := $(shell command -v wails 2> /dev/null || echo $(GOPATH)/bin/wails)

.PHONY: i dev build build-linux bump patch package-deb package-rpm package-archlinux package-linux clean

i:
	cd frontend && pnpm install

dev:
	$(WAILS) dev

build:
	@echo "Building PortFinder v$(VERSION)"
	$(WAILS) build -ldflags "-X main.Version=$(VERSION)"

build-linux:
	@echo "Building PortFinder for Linux v$(VERSION)"
	$(WAILS) build -platform linux/amd64 -ldflags "-X main.Version=$(VERSION)"

# bump: set version to today's date (new day release)
bump:
	@NEW_VERSION=$$(date +"%Y.%-m.%-d"); \
	echo "$$NEW_VERSION" > version.txt; \
	sed -i '' 's/"productVersion": ".*"/"productVersion": "'$$NEW_VERSION'"/' wails.json; \
	echo "Version bumped to $$NEW_VERSION"

# patch: increment the patch number for today's version (e.g., 2026.4.13 -> 2026.4.13-1, 2026.4.13-1 -> 2026.4.13-2)
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
	sed -i '' 's/"productVersion": ".*"/"productVersion": "'$$NEW_VERSION'"/' wails.json; \
	echo "Version patched to $$NEW_VERSION"

package-deb: build-linux
	VERSION=$(VERSION) nfpm package --packager deb --target dist/

package-rpm: build-linux
	VERSION=$(VERSION) nfpm package --packager rpm --target dist/

package-archlinux: build-linux
	VERSION=$(VERSION) nfpm package --packager archlinux --target dist/

package-linux: package-deb package-rpm package-archlinux

clean:
	rm -rf build/bin dist/

.DEFAULT_GOAL := build
