#!/bin/sh

# Configuration file
CONFIG_PATH="$RELAYER_DIR"/"$CONFIG"
echo Config: "$CONFIG_PATH"

echo "Setting up relayer for chains:"
echo => Chain: "$CHAIN_A" [$CHAIN_A_HOME]
echo => Chain: "$CHAIN_B" [$CHAIN_B_HOME]
echo Waiting 30 seconds for chains to generate blocks...
sleep 30
echo Done waiting, proceeding...

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Show relayer version"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly version

echo "================================================================================================================="
echo "                                                CONFIGURATION                                                    "
echo "================================================================================================================="

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Add light clients configuration for chains"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" light add tcp://"$CHAIN_A":26657 -c "$CHAIN_A" -s "$CHAIN_A_HOME" -p -y --force
sleep 3
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" light add tcp://"$CHAIN_B":26557 -c "$CHAIN_B" -s "$CHAIN_B_HOME" -p -y --force
sleep 3
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" light add tcp://"$CHAIN_B":26557 -c "$CHAIN_A" -s "$CHAIN_A_HOME" -y --force
sleep 3
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" light add tcp://"$CHAIN_A":26657 -c "$CHAIN_B" -s "$CHAIN_B_HOME" -y --force
sleep 3
echo "-----------------------------------------------------------------------------------------------------------------"
echo "Add keys for chains"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" keys add "$CHAIN_A" "$CHAIN_A_HOME"/key_seed.json
rrly -c "$CONFIG_PATH" keys add "$CHAIN_B" "$CHAIN_B_HOME"/key_seed.json

echo "================================================================================================================="
echo "                                             CLIENTS                                                             "
echo "================================================================================================================="

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Create client transactions"
echo "-----------------------------------------------------------------------------------------------------------------"
echo "creating "$CHAIN_B"_client on chain "$CHAIN_A"..."
rrly -c "$CONFIG_PATH" tx raw create-client "$CHAIN_A" "$CHAIN_B" "$CHAIN_B"_client
echo "-----------------------------------------------------------------------------------------------------------------"
echo "creating "$CHAIN_A"_client on chain "$CHAIN_B"..."
rrly -c "$CONFIG_PATH" tx raw create-client "$CHAIN_B" "$CHAIN_A" "$CHAIN_A"_client

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Query clients"
echo "-----------------------------------------------------------------------------------------------------------------"
echo "querying "$CHAIN_B"_client on chain "$CHAIN_A"..."
rrly -c "$CONFIG_PATH" query client state "$CHAIN_A" "$CHAIN_B"_client
echo "-----------------------------------------------------------------------------------------------------------------"
echo "querying "$CHAIN_A"_client on chain "$CHAIN_B"..."
rrly -c "$CONFIG_PATH" query client state "$CHAIN_B" "$CHAIN_A"_client

echo "================================================================================================================="
echo "                                             CONNECTIONS                                                         "
echo "================================================================================================================="

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Connection Init transaction"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" tx raw conn-init \
        "$CHAIN_A" \
        "$CHAIN_B" \
        "$CHAIN_B"_client \
        "$CHAIN_A"_client \
        conn_"$CHAIN_A"_to_"$CHAIN_B" \
        conn_"$CHAIN_B"_to_"$CHAIN_A"

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Connection Open Try transaction"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" tx raw conn-try \
        "$CHAIN_B" \
        "$CHAIN_A" \
        "$CHAIN_A"_client \
        "$CHAIN_B"_client \
        conn_"$CHAIN_B"_to_"$CHAIN_A" \
        conn_"$CHAIN_A"_to_"$CHAIN_B"

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Connection Open Ack transaction"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" tx raw conn-ack \
        "$CHAIN_A" \
        "$CHAIN_B" \
        "$CHAIN_B"_client \
        "$CHAIN_A"_client \
        conn_"$CHAIN_A"_to_"$CHAIN_B" \
        conn_"$CHAIN_B"_to_"$CHAIN_A"

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Connection Open Confirm transaction"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" tx raw conn-confirm \
        "$CHAIN_B" \
        "$CHAIN_A" \
        "$CHAIN_A"_client \
        "$CHAIN_B"_client \
        conn_"$CHAIN_B"_to_"$CHAIN_A" \
        conn_"$CHAIN_A"_to_"$CHAIN_B"

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Query connection - Verify that the two ends are in Open state"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" query connection end "$CHAIN_A" conn_"$CHAIN_A"_to_"$CHAIN_B"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" query connection end "$CHAIN_B" conn_"$CHAIN_B"_to_"$CHAIN_A"

echo "================================================================================================================="
echo "                                                CHANNELS                                                         "
echo "================================================================================================================="

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Channel Open Init transaction"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" tx raw chan-init \
        "$CHAIN_A" \
        "$CHAIN_B" \
        conn_"$CHAIN_A"_to_"$CHAIN_B" \
        transfer \
        transfer \
        chan_"$CHAIN_A"_to_"$CHAIN_B" \
        chan_"$CHAIN_B"_to_"$CHAIN_A"

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Channel Open Try transaction"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH"  tx raw chan-try \
        "$CHAIN_B" \
        "$CHAIN_A" \
        conn_"$CHAIN_B"_to_"$CHAIN_A" \
        transfer \
        transfer \
        chan_"$CHAIN_B"_to_"$CHAIN_A" \
        chan_"$CHAIN_A"_to_"$CHAIN_B"

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Channel Open Try transaction"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" tx raw chan-ack \
        "$CHAIN_A" \
        "$CHAIN_B" \
        conn_"$CHAIN_A"_to_"$CHAIN_B" \
        transfer \
        transfer \
        chan_"$CHAIN_A"_to_"$CHAIN_B" \
        chan_"$CHAIN_B"_to_"$CHAIN_A"

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Channel Open Confirm transaction"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH"  tx raw chan-confirm \
        "$CHAIN_B" \
        "$CHAIN_A" \
        conn_"$CHAIN_B"_to_"$CHAIN_A" \
        transfer \
        transfer \
        chan_"$CHAIN_B"_to_"$CHAIN_A" \
        chan_"$CHAIN_A"_to_"$CHAIN_B"

echo "-----------------------------------------------------------------------------------------------------------------"
echo "Query channel - Verify that the two ends are in Open state"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" query channel end "$CHAIN_A" transfer chan_"$CHAIN_A"_to_"$CHAIN_B"
echo "-----------------------------------------------------------------------------------------------------------------"
rrly -c "$CONFIG_PATH" query channel end "$CHAIN_B" transfer chan_"$CHAIN_B"_to_"$CHAIN_A"