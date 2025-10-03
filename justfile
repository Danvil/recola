# Shell for Linux
#set shell := ["sh", "-c"]

# Shell for Windows
set windows-shell := ["pwsh", "-NoLogo", "-Command"]

test:
  cargo nextest run --release --all-features
  cargo test --release --doc --all-features

format:
  cargo +nightly fmt

check:
  cargo check

eph:
	cargo run --release -p eph

eph_tracy:
    # Note there seems to be an issue with tracing but this works:
    $env:TRACY_CLIENT_SYS_CXXFLAGS = "/DRelationProcessorDie=((LOGICAL_PROCESSOR_RELATIONSHIP)5) /DRelationProcessorModule=((LOGICAL_PROCESSOR_RELATIONSHIP)7)"
    cargo run --release --features profile-with-tracy -p eph

[windows]
deploy_goscl:
	cargo build --release -p goscl
	scripts/deploy_goscl.ps1

[windows]
deploy_gosrv:
	cargo build --release -p gosrv
	scripts/deploy_gosrv.ps1

[windows]
[working-directory: 'deploy/main/goscl']
goscl:
	./goscl.exe

[windows]
[working-directory: 'deploy/main/gosrv']
gosrv:
	./gosrv.exe

deploy_and_goscl: deploy_goscl goscl

deploy_and_gosrv: deploy_gosrv gosrv

alias gos := deploy_and_goscl
alias srv := deploy_and_gosrv

[windows]
publish password: deploy_goscl
	steamworks_sdk\steamworks_sdk_162\sdk\tools\ContentBuilder\builder\steamcmd.exe +login steamup_bot_ikabur {{password}} +run_app_build ..\..\..\..\..\..\scripts\app_build_3893320.vdf +quit
