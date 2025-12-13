.PHONY: dev build check clean

dev:
	npm run tauri dev

build:
	npm run tauri build

check:
	npm run build
	cd src-tauri && cargo check

clean:
	rm -rf dist src-tauri/target
