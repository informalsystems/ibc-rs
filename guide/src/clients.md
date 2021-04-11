# Client

## Table of Contents
<!-- toc -->

## Create Client
Use the `create client` command to create a new client.

```shell
USAGE:
    hermes create client <OPTIONS>

DESCRIPTION:
    Create a new IBC client

POSITIONAL ARGUMENTS:
    dst_chain_id              identifier of the destination chain
    src_chain_id              identifier of the source chain
```

__Example__

Create a new client of `ibc-1` on `ibc-0`:

```shell
hermes create client ibc-0 ibc-1 | jq
```

```json
{
  "result": {
    "CreateClient": {
      "client_id": "07-tendermint-0",
      "client_type": "Tendermint",
      "consensus_height": {
        "revision_height": 355,
        "revision_number": 1
      },
      "height": {
        "revision_height": 568,
        "revision_number": 0
      }
    }
  },
  "status": "success"
}
```

A new client is created with identifier `07-tendermint-3`

## Update Client
Use the `update client` command to update an existing client with a new consensus state.
Specific update and trusted heights can be specified.

```shell
USAGE:
    hermes update client <OPTIONS>

DESCRIPTION:
    Update an IBC client

POSITIONAL ARGUMENTS:
    dst_chain_id              identifier of the destination chain
    dst_client_id             identifier of the client to be updated on destination chain

FLAGS:
    -h, --target-height TARGET-HEIGHT
    -t, --trusted-height TRUSTED-HEIGHT
```

__Example__

Update the client on `ibc-0` with latest header of `ibc-1`

```shell
hermes update client ibc-0 07-tendermint-0  | jq
```

```json
{
  "result": {
    "UpdateClient": {
      "common": {
        "client_id": "07-tendermint-0",
        "client_type": "Tendermint",
        "consensus_height": {
          "revision_height": 273,
          "revision_number": 1
        },
        "height": {
          "revision_height": 280,
          "revision_number": 0
        }
      },
      "header": {
        "Tendermint": {
        ...
      }
    }
  },
  "status": "success"
}
```

The client with identifier `07-tendermint-0` has been updated with the consensus state at height `1-273`.