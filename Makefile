# Do not change this starting part, used by mod.rs
UNAME := $(shell uname)

ifeq ($(UNAME), Linux)
ARCH := elf64
RUST_TARGET := x86_64-unknown-linux-gnu
endif
ifeq ($(UNAME), Darwin)
ARCH := macho64
RUST_TARGET := x86_64-apple-darwin
endif

tests/%.run: tests/%.s runtime/start.rs
	nasm -f $(ARCH) tests/$*.s -o tests/$*.o
	ar rcs tests/lib$*.a tests/$*.o
	rustc --target $(RUST_TARGET) -L tests/ -lour_code:$* runtime/start.rs -o tests/$*.run


# Change below to whatever might be helpful! 
MODE ?= -c

tests/%.s: tests/%.snek src/main.rs
	cargo run --target $(RUST_TARGET) -- $(MODE) $< tests/$*.s

clean:
	cargo clean
	rm -f tests/*.a tests/*.s tests/*.run tests/*.o

.PHONY: test
test:
	cargo build --target $(RUST_TARGET)
	cargo test -- --test-threads=1

