# Convenience targets. Requires the Dioxus CLI (`dx`, 0.7.x) and the
# wasm32-unknown-unknown target (`rustup target add wasm32-unknown-unknown`).

APP := crates/app

.PHONY: web desktop serve-web run-desktop test lint renders bench calibrate spectrograms clean

## Serve the web build with hot reload on http://127.0.0.1:8080
serve-web:
	cd $(APP) && dx serve --web --release

## Run the native Linux app (PulseAudio / PipeWire-pulse)
run-desktop:
	cd $(APP) && dx serve --desktop --release

## Release bundles: target/dx/wunderkammer/release/{web/public,linux/app}
web:
	cd $(APP) && dx build --web --release

desktop:
	cd $(APP) && dx build --desktop --release

test:
	cargo test --release -p wunderkammer-dsp -p wunderkammer

lint:
	cargo fmt --all --check
	cargo clippy --all-targets -p wunderkammer-dsp -p wunderkammer-worklet -- -D warnings
	cargo clippy --all-targets -p wunderkammer --features desktop -- -D warnings
	cargo clippy -p wunderkammer --features web --target wasm32-unknown-unknown -- -D warnings

## Render every factory preset to WAV files in ./renders
renders:
	cargo run --release -p wunderkammer-dsp --example render -- renders

bench:
	cargo run --release -p wunderkammer-dsp --example bench

## Loudness survey of every object across pitch/time/macro settings
## (`make calibrate OBJECT=BALLOON` lists every setting of one object)
calibrate:
	cargo run --release -p wunderkammer-dsp --example calibrate -- $(OBJECT)

## Spectrograms of the preset renders (needs python3 with numpy + matplotlib)
spectrograms: renders
	python3 tools/spectrogram.py renders/spectrograms.png renders/*.wav

clean:
	cargo clean
