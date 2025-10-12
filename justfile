# Shell for Linux
set shell := ["sh", "-c"]

# Shell for Windows
set windows-shell := ["pwsh", "-NoLogo", "-Command"]

test:
  cargo nextest run --release --all-features
  cargo test --release --doc --all-features

format:
  cargo +nightly fmt

check:
  cargo check

recola: recola_package_assets
    cargo run --release -p recola
    #$env:TRACY_CLIENT_SYS_CXXFLAGS = "/DRelationProcessorDie=((LOGICAL_PROCESSOR_RELATIONSHIP)5) /DRelationProcessorModule=((LOGICAL_PROCESSOR_RELATIONSHIP)7)"
    #cargo run --release -p recola --features profile-with-tracy
    # cargo run --release -p recola --features disco

recola_export_blend:
    python .\scripts\blender_export_driver.py

recola_package_assets: recola_export_blend
    python .\scripts\package_release.py --package-assets

recola_release: recola_export_blend
    python .\scripts\package_release.py --package-assets --create-release
