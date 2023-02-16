## Building
```
cd program
cargo build-sbf
```

## Unit Testing
```
cd program

cargo test-sbf -- --test-threads=1 --nocapture
cargo test-sbf test_mint_transfer_pass -- --test-threads=1 --nocapture

cargo test-sbf test_wallet_new -- --test-threads=1 --nocapture
```