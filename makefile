all: .build
clean:
	cargo clean
build:
	docker build --rm -t rs-net-radio-m:latest .
