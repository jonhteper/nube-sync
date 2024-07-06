OUTDIR := ./package
BIN := nubesync

all: prepare release

prepare: 
	mkdir $(OUTDIR) || true
	cp nube-sync.config.toml $(OUTDIR) || true

release:
	cargo build --release --target=x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/$(BIN) $(OUTDIR)

dev:
	cargo build
	cp target/debug/$(BIN) $(OUTDIR)

lint:
	cargo clippy

flagged-release:
	mkdir $(OUTDIR)/flagged
	cargo build --release --target=x86_64-unknown-linux-musl --features version_migration
	cp target/x86_64-unknown-linux-musl/release/$(BIN) $(OUTDIR)/flagged
