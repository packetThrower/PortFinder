VERSION := $(shell cat version.txt)
GOPATH ?= $(shell go env GOPATH)
WAILS := $(shell command -v wails 2> /dev/null || echo $(GOPATH)/bin/wails)

.PHONY: i dev build build-linux bump package-deb package-rpm package-archlinux package-linux clean

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

bump:
	@NEW_VERSION=$$(date +"%Y.%-m.%-d"); \
	echo "$$NEW_VERSION" > version.txt; \
	sed -i '' 's/"productVersion": ".*"/"productVersion": "'$$NEW_VERSION'"/' wails.json; \
	echo "Version bumped to $$NEW_VERSION"

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
