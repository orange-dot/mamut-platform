.PHONY: all firmware services dashboard sim test clean proto

all: firmware services dashboard

firmware:
	cd firmware && cmake -B build -DCMAKE_BUILD_TYPE=Debug && cmake --build build

services:
	cd services && cargo build

dashboard:
	cd dashboard && npm run build

sim:
	cd sim && pip install -e . && pytest

proto:
	cd tools && ./proto-gen.sh

test: test-firmware test-services test-dashboard test-sim

test-firmware:
	cd firmware && cmake -B build -DCMAKE_BUILD_TYPE=Debug && cmake --build build --target test

test-services:
	cd services && cargo test

test-dashboard:
	cd dashboard && npm test

test-sim:
	cd sim && pytest

clean:
	rm -rf firmware/build services/target dashboard/dist

format:
	cd services && cargo fmt
	cd dashboard && npm run format

lint:
	cd services && cargo clippy
	cd dashboard && npm run lint
