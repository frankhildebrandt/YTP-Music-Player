DEV_PORT ?= 1420

.PHONY: help install run web-run preview build check cargo-check bundle bundle-windows clean free-dev-port

help:
	@echo "YTP Title Catcher targets:"
	@echo "  make install      Install Node dependencies"
	@echo "  make run          Run the Tauri desktop app in development mode"
	@echo "  make web-run      Run only the Vite web UI"
	@echo "  make preview      Preview the production web build"
	@echo "  make build        Build the frontend"
	@echo "  make check        Typecheck/build frontend and check Rust"
	@echo "  make bundle       Create local macOS Tauri release bundles"
	@echo "  make bundle-windows"
	@echo "                    Create Windows bundles on a Windows host"
	@echo "  make clean        Remove generated build output"

install:
	npm install

run: free-dev-port
	npm run tauri:dev

web-run:
	npm run dev

free-dev-port:
	@pid="$$(lsof -tiTCP:$(DEV_PORT) -sTCP:LISTEN)"; \
	if [ -n "$$pid" ]; then \
		echo "Stopping process on port $(DEV_PORT): $$pid"; \
		kill $$pid; \
		sleep 1; \
	fi

preview:
	npm run preview

build:
	npm run build

cargo-check:
	cd src-tauri && cargo check

check: build cargo-check

bundle:
	npm run tauri:build

bundle-windows:
	npm run tauri:build:windows

clean:
	rm -rf dist src-tauri/target
