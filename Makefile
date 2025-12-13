.PHONY: dev build check fmt lint test

dev:
	npm run tauri dev

build:
	npm run tauri build

check:
	npm run build
	cd src-tauri && cargo check

fmt:
	npx prettier --write src/
	cd src-tauri && cargo fmt

lint:
	npx eslint src/
	cd src-tauri && cargo clippy

test:
	cd src-tauri && cargo test
