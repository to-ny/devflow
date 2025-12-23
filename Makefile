.PHONY: dev build build-dev build-windows build-windows-dev check fmt lint test

dev:
	npm run tauri dev

build:
	npm run tauri build

build-dev:
	npm run tauri build -- --features devtools

build-windows:
	npm run tauri build -- --target x86_64-pc-windows-msvc

build-windows-dev:
	npm run tauri build -- --target x86_64-pc-windows-msvc --features devtools

check:
	npm run build
	cd src-tauri && cargo check

fmt:
	npx prettier --write src/
	cd src-tauri && cargo fmt

lint:
	npx eslint src/
	cd src-tauri && cargo clippy -- -D warnings

test:
	npm run test
	cd src-tauri && cargo test
