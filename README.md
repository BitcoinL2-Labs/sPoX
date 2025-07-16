# sPoX

## Demo flow

sPox can be tested with the sBTC devenv:
 - `make devenv-up`, wait for nakamoto and `./signers.sh demo` to get the signers ready
 - Get signers aggregate key with `cargo run -p signer --bin demo-cli info` (from sBTC)
 - Edit `signer/src/bin/demo_cli.rs`, `exec_deposit` to return after `send_raw_transaction` but before `create_deposit`
 
Now, in no particular order:
 - Start spox: `cargo run -- -c src/config/default.toml --signers-xonly <signers xonly pubkey from info above>`
 - Create a deposit (without notifying emily): `cargo run -p signer --bin demo-cli deposit --amount 123456` (from sBTC)

This will look for deposits made to the signers pubkey with the devenv default values. Once the tx is confirmed it should appear on Emily, assuming it didn't expire in the meantime, and be processed by the signers, assuming the amount is not too low to be ignored.
