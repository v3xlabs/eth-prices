export RPC_URL := "https://ethereum.reth.rs/rpc"

default:
    just --list

check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo doc --all-features --no-deps

alias fix := format
alias fmt := format
format:
    cargo fmt
    cargo clippy --fix --allow-dirty

test:
    cargo test
    pnpm --sequential -r test

build:
    cd pkg && cargo build
    cd ts && pnpm build

eval:
    pnpm --dir bench eval

eval-rust:
    cargo run --release -p eth-prices-bench

eval-ts:
    pnpm --dir ts build
    pnpm --dir bench eval:ts
