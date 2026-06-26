export RPC_URL := "https://ethereum.reth.rs/rpc"

default:
    just --list

test:
    cargo test
    pnpm --sequential -r test

build:
    cargo build
    pnpm -r build
