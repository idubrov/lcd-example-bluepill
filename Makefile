CARGO = xargo

CARGO_OPTS =
TARGET = thumbv7m-none-eabi
NAME = lcd-example-bluepill

all:
	$(MAKE) build
	$(MAKE) doc

build:
	$(CARGO) $(CARGO_OPTS) build

build.rel:
	$(CARGO) $(CARGO_OPTS) build --release

clean:
	$(CARGO) $(CARGO_OPTS) clean

check: build test

test:
	$(CARGO) $(CARGO_OPTS) test

bench:
	$(CARGO) $(CARGO_OPTS) bench

doc:
	$(CARGO) $(CARGO_OPTS) doc

program: build
	openocd -f interface/stlink-v2.cfg -f target/stm32f1x.cfg \
		-c "program target/$(TARGET)/debug/$(NAME) verify reset exit"

program.rel: build.rel
	openocd -f interface/stlink-v2.cfg -f target/stm32f1x.cfg \
		-c "program target/$(TARGET)/release/$(NAME) verify reset exit"

.PHONY: all build build.rel clean check test bench doc program program.rel
