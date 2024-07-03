OUTDIR := ./package
BIN := nubesync

all: prepare release

prepare: 
	mkdir $(OUTDIR) || true
	cp nube-sync.config.toml $(OUTDIR) || true

release:
	cargo build --release
	cp target/release/$(BIN) $(OUTDIR)

dev:
	cargo build
	cp target/debug/$(BIN) $(OUTDIR)

lint:
	cargo clippy
