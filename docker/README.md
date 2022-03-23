# (Optional: Build on host) Build cross-compile binary

```
export CC_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-gcc
export CXX_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-g++                                  
export AR_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-ar                                    
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc                  
cargo build --release --target=x86_64-unknown-linux-gnu
```

# Build docker image

```
docker build -t sip-procyon -f docker/Dockerfile .

# Or already build from host
docker build -t sip-procyon -f docker/Dockerfile.host .

AWS_ECR_URI=679762087785.dkr.ecr.ap-southeast-1.amazonaws.com
ZONE=ap-southeast-1
aws ecr get-login-password --region $ZONE | docker login --username AWS --password-stdin $AWS_ECR_URI

TAGS=${AWS_ECR_URI}/sip-procyon
docker tag sip-procyon $TAGS
docker push $TAGS
```

# Generate accounts's sessions and genesis file

```
mkdir -p workspace
base_dir=workspace

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

command="docker run \
  -v $(pwd)/$base_dir:/tmp \
  --rm sip-procyon \
  bootstrap-chain \
  --chain-id "43" \
  --chain-name "Procyon" \
  --token-symbol "SIP" \
  --base-path /tmp \
  $list_account_ids_parameter \
  --sudo-account-id $sudo_account_id"
  
eval $command \
   > "$base_dir/chainspec.json"
```

# Update genesis faucet accounts

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

# Run nodes

```
account_ids=($(find workspace/ -mindepth 1 -maxdepth 1 -type d -exec basename {} \;))
telemetry_url=wss://telemetry.hoalv.tk/submit/

docker network create substrate

docker run -d \
      --restart always \
      -v $(pwd):/tmp \
      --name $account_id \
      --log-opt max-size=5m \
      --log-opt max-file=1 \
      -p 9933:9933 \
      -p 9944:9944 \
      -p 30334:30334 \
      sip-procyon \
      --name $account_id \
      --validator \
      --chain "/tmp/chainspec.json" \
      --base-path "/tmp/$account_id" \
      --rpc-cors=all \
      --rpc-port 9933 \
      --ws-port 9944 \
      --unsafe-rpc-external \
      --unsafe-ws-external \
      --port 30334 \
      --discover-local \
      --allow-private-ipv4 \
      --telemetry-url "$telemetry_url 0" \
      --node-key-file "/tmp/$account_id/p2p_key" \
      --execution Native
done
```

# Remove containers

```
for account_id in $account_ids; do
  docker stop $account_id
  docker rm $account_id
done
```