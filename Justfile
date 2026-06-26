export RPC_URL := "https://ethereum.reth.rs/rpc"

default:
    just --list

test:
    cargo test
    pnpm --sequential -r test

build:
    cd pkg && cargo build
    cd pkg && pnpm build
    cd ts && pnpm build
