# Solana Merkle Tree: Program, CLI client
Solana program which show basic merkle tree operations(compute root-hash, insert leaf hash).

## Setup program
### Test
```sh
cargo test-sbf -- --nocapture
```

### Build
```sh
cargo-build-sbf
```

### Deploy
```sh
solana program deploy --program-id ./target/deploy/merkle_tree_program-keypair.json ./target/deploy/merkle_tree_program.so
```
