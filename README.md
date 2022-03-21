# Build source

```
cargo build --release
```

# Run node as development mode

- Only Alith is validator (produce and finalize block)
- Expose RPC port to all

```
# If you have already build source
./target/release/substrate --dev

# Or
cargo run --release -- --dev
```

# Generate genesis file

```
mkdir -p workspace
base_dir=$(pwd)/workspace

list_account_ids_parameter=""

# Replace by list of validators
account_ids=(
    9caE3915fb466b6c86E25fd3a8E333c4F9Addd03:F8Ff983526Dd27f34ACC51fFB035922879D7F9c2
    7EF085eAE9e99c379aB03B9707D2Adfe133A4b3A:ff354ed3972939c1d3fd09A9084b87Ca1Ca8bd7F
    d113F64E5AAcec946673619D933EA0F7EF80e27b:D218f1Bb54220a57679a464bf50648E20F209c73
)
for account_id in $account_ids; do
  list_account_ids_parameter+="--authority-ids $account_id "
done

# Replace by sudo account id
sudo_account_id=9caE3915fb466b6c86E25fd3a8E333c4F9Addd03

command="./target/release/substrate bootstrap-chain \
  --chain-id "43" \
  --chain-name "Procyon" \
  --token-symbol "SIP" \
  --base-path $base_dir \
  $list_account_ids_parameter \
  --sudo-account-id $sudo_account_id"
  
eval $command \
   > "$base_dir/chainspec.json"
```

# Update balance for accounts in genesis file

- Remember update validator stash accounts

```
"balances": {
  "balances": [
    [
      "0x9caE3915fb466b6c86E25fd3a8E333c4F9Addd03",
      1000000000000000000000000000
    ],
    [
      "0xF8Ff983526Dd27f34ACC51fFB035922879D7F9c2",
      1000000000000000000000000000
    ],
    [
      "0x7EF085eAE9e99c379aB03B9707D2Adfe133A4b3A",
      1000000000000000000000000000
    ],
    [
      "0xff354ed3972939c1d3fd09A9084b87Ca1Ca8bd7F",
      1000000000000000000000000000
    ],
    [
      "0xd113F64E5AAcec946673619D933EA0F7EF80e27b",
      1000000000000000000000000000
    ],
    [
      "0xD218f1Bb54220a57679a464bf50648E20F209c73",
      1000000000000000000000000000
    ]
  ]
}
```

# Run node from generated genesis

- **Bash version > 3**

```
account_ids=(
  9caE3915fb466b6c86E25fd3a8E333c4F9Addd03
  7EF085eAE9e99c379aB03B9707D2Adfe133A4b3A
  d113F64E5AAcec946673619D933EA0F7EF80e27b
)

for i in "${!account_ids[@]}"; do
  nohup ./target/release/substrate \
    --validator \
    --chain "$base_dir/chainspec.json" \
    --base-path "$base_dir/${account_ids[$i]}" \
    --rpc-cors=all \
    --rpc-port $(( 9933 + $i )) \
    --ws-port $(( 9944 + $i )) \
    --unsafe-rpc-external \
    --unsafe-ws-external \
    --port $(( 30334 + $i )) \
    --discover-local \
    --no-telemetry \
    --allow-private-ipv4 \
    --execution Wasm \
    > $base_dir/node_$i.log &
done
```

- **Zsh**

```
account_ids=(
  9caE3915fb466b6c86E25fd3a8E333c4F9Addd03
  7EF085eAE9e99c379aB03B9707D2Adfe133A4b3A
  d113F64E5AAcec946673619D933EA0F7EF80e27b
)

for i in {1..$#account_ids}; do
  nohup ./target/release/substrate \
    --validator \
    --chain "$base_dir/chainspec.json" \
    --base-path "$base_dir/${account_ids[$i]}" \
    --rpc-cors=all \
    --rpc-port $(( 9933 + $i )) \
    --ws-port $(( 9944 + $i )) \
    --unsafe-rpc-external \
    --unsafe-ws-external \
    --port $(( 30334 + $i )) \
    --discover-local \
    --no-telemetry \
    --allow-private-ipv4 \
    --execution Wasm \
    > $base_dir/node_$i.log &
done
```

- **Single node**

```
account_id=9caE3915fb466b6c86E25fd3a8E333c4F9Addd03
./target/release/substrate \
    --validator \
    --chain "$base_dir/chainspec.json" \
    --base-path "$base_dir/$account_id" \
    --rpc-cors=all \
    --rpc-port 9933 \
    --ws-port 9944 \
    --unsafe-rpc-external \
    --unsafe-ws-external \
    --port 30334 \
    --discover-local \
    --no-telemetry \
    --allow-private-ipv4 \
    --execution Wasm
```

# Purge chain

```
for i in "${!account_ids[@]}"; do
  ./target/release/substrate purge-chain 
      --chain "$base_dir/chainspec.json" \
      --base-path "$base_dir/${account_ids[$i]}"
done
```

```
for i in {1..$#account_ids}; do
  ./target/release/substrate purge-chain \
      --chain "$base_dir/chainspec.json" \
      --base-path "$base_dir/${account_ids[$i]}"
done
```