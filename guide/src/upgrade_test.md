1. Patch the `one-chain` script of rrly:


```diff
diff --git a/scripts/one-chain b/scripts/one-chain
index d0995fe..3702a88 100755
--- a/scripts/one-chain
+++ b/scripts/one-chain
@@ -99,6 +99,7 @@ if [ $platform = 'linux' ]; then
   sed -i 's/index_all_keys = false/index_all_keys = true/g' $CHAINDIR/$CHAINID/config/config.toml
   # sed -i '' 's#index-events = \[\]#index-events = \["message.action","send_packet.packet_src_channel","send_packet.packet_sequence"\]#g' $CHAINDIR/$CHAINID/config/app.toml
 else
+  sed -i '' 's#"172800s"#"200s"#g' $CHAINDIR/$CHAINID/config/genesis.json
   sed -i '' 's#"tcp://127.0.0.1:26657"#"tcp://0.0.0.0:'"$RPCPORT"'"#g' $CHAINDIR/$CHAINID/config/config.toml
   sed -i '' 's#"tcp://0.0.0.0:26656"#"tcp://0.0.0.0:'"$P2PPORT"'"#g' $CHAINDIR/$CHAINID/config/config.toml
   sed -i '' 's#"localhost:6060"#"localhost:'"$P2PPORT"'"#g' $CHAINDIR/$CHAINID/config/config.toml
```


2. Start two gaia instances using the patched developer environment:


```shell
./scripts/two-chainz
```

3. Setup the Go relayer for these chains:
```shell
rly tx link demo -d -o 3s
```

Check that everything went fine so far:

```shell
$ rly paths list
 0: demo                 -> chns(✔) clnts(✔) conn(✔) chan(✔) (ibc-0:transfer<>ibc-1:transfer)
```

4. Create the upgrade plan for chain ibc-0:

It's important that we parametrize the upgrade plan with a height parameter that
is at least 300 heights ahead of the current height of chain ibc-0.

First, obtain the current height:
```shell
gaiad query block | jq | grep height
      "height": "470",
```

Now create the upgrade plan for height 800:
```shell
echo '{
  "Name": "test",
  "Height": 800,
  "Info": ""
}' > ./upgrade-plan.json
```

5. Query for the upgrade plan, check that it was submitted correctly

```shell
$ gaiad query gov proposal 1 --home data/ibc-0/

content:
  '@type': /cosmos.upgrade.v1beta1.SoftwareUpgradeProposal
  description: upgrade the chain's software and unbonding period
  plan:
    height: "800"
    info: ""
    name: test
....
proposal_id: "1"
status: PROPOSAL_STATUS_VOTING_PERIOD
submit_time: "2021-03-08T13:07:01.417163Z"
total_deposit:
- amount: "10000000"
  denom: stake
voting_end_time: "2021-03-08T13:10:21.417163Z"
voting_start_time: "2021-03-08T13:07:01.417163Z"
```

6. Vote on the proposal

The parameter "1" should match the "proposal_id:" from the upgrade proposal
we submitted at step 5.

```shell
gaiad tx gov vote 1 yes --home data/ibc-0/data/ --keyring-backend test --keyring-dir data/ibc-0/ --chain-id ibc-0 --from validator
```

Once ibc-0 reaches height 800, it should stop executing.


7. Initialize and test Hermes


```shell
./scripts/init-clients ~/.hermes/config.toml ibc-0 ibc-1
    Building the Rust relayer...
    Removing light client peers from configuration...
    Adding primary peers to light client configuration...
    Adding secondary peers to light client configuration...
    Importing keys...
    Done!
```

The following command should output the upgraded client and consensus states
with their proofs:
```shell
hermes tx raw upgrade-client ibc-1 ibc-0 07-tendermint-0
```