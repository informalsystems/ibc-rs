#!/usr/bin/env python3

from typing import Any, List, Optional, TypedDict, TypeVar, Generic, Type, Callable, Tuple, NewType

import os
import json
import argparse
import subprocess

import logging as l

from time import sleep
from enum import Enum
from pathlib import Path
from dataclasses import dataclass, fields as datafields, is_dataclass


class ExpectedSuccess(Exception):
    cmd: 'Cmd'
    status: str
    result: Any

    def __init__(self, cmd: 'Cmd', status: str, result: Any) -> None:
        self.cmd = cmd
        self.status = status
        self.result = result

        super().__init__(
            f"Command '{cmd}' failed. Expected 'success', got '{status}'. Message: {result}"
        )


T = TypeVar('T')


@dataclass
class CmdResult(Generic[T]):
    cmd: 'Cmd'
    result: Any

    def success(self) -> T:
        status = self.result.get('status') or 'unknown'
        result = self.result.get('result') or {}

        if status == "success":
            return self.cmd.process(result)
        else:
            raise ExpectedSuccess(self.cmd, status, result)


class Cmd(Generic[T]):
    name: str

    def process(self, result: Any) -> Any:
        raise NotImplementedError("Cmd::process")

    def args(self) -> List[str]:
        raise NotImplementedError("Cmd::args")

    def to_cmd(self) -> str:
        return f"{self.name} {' '.join(self.args())}"

    def run(self, config: str) -> CmdResult[T]:
        full_cmd = f'cargo run --bin relayer -- -c {config}'.split(' ')
        full_cmd.extend(self.name.split(' '))
        full_cmd.extend(self.args())
        l.debug(' '.join(full_cmd))

        res = subprocess.run(full_cmd, capture_output=True, text=True)
        lines = res.stdout.splitlines()
        last_line = ''.join(lines[-1:])
        l.debug(last_line)

        return CmdResult(cmd=self, result=json.loads(last_line))


C = TypeVar('C', bound=Cmd)


def cmd(name: str) -> Callable[[Type[C]], Type[C]]:
    def decorator(klass: Type[C]) -> Type[C]:
        klass.name = name
        return klass

    return decorator


def from_dict(klass, dikt) -> Any:
    if is_dataclass(klass):
        fields = datafields(klass)
        args = {f.name: from_dict(f.type, dikt[f.name]) for f in fields}
        return klass(**args)
    else:
        return dikt


# =============================================================================

@dataclass
class Height:
    revision_height: int
    revision_number: int


@dataclass
class Duration:
    nanos: int
    secs: int


@dataclass
class TrustLevel:
    denominator: int
    numerator: int


PortId = NewType('PortId', str)
ChainId = NewType('ChainId', str)
ClientId = NewType('ClientId', str)
ChannelId = NewType('ChannelId', str)
ConnectionId = NewType('ConnectionId', str)

Sequence = NewType('Sequence', str)
ClientType = NewType('ClientType', str)
BlockHeight = NewType('BlockHeight', str)


class Ordering(Enum):
    UNORDERED = 'UNORDERED'
    ORDERED = 'ORDERED'

# =============================================================================


@dataclass
class ClientCreated:
    client_id: ClientId
    client_type: ClientType
    consensus_height: Height
    height: BlockHeight


@dataclass
@cmd("tx raw create-client")
class TxCreateClient(Cmd[ClientCreated]):
    dst_chain_id: ChainId
    src_chain_id: ChainId

    def args(self) -> List[str]:
        return [self.dst_chain_id, self.src_chain_id]

    def process(self, result: Any) -> ClientCreated:
        return from_dict(ClientCreated, result[0]['CreateClient'])


# -----------------------------------------------------------------------------

@dataclass
class ClientUpdated:
    client_id: ClientId
    client_type: ClientType
    consensus_height: Height
    height: BlockHeight


@dataclass
@cmd("tx raw update-client")
class TxUpdateClient(Cmd[ClientUpdated]):
    dst_chain_id: ChainId
    src_chain_id: ChainId
    dst_client_id: ClientId

    def args(self) -> List[str]:
        return [self.dst_chain_id, self.src_chain_id, self.dst_client_id]

    def process(self, result: Any) -> ClientUpdated:
        return from_dict(ClientUpdated, result[0]['UpdateClient'])


# -----------------------------------------------------------------------------


@dataclass
class ClientState:
    allow_update_after_expiry: bool
    allow_update_after_misbehaviour: bool
    chain_id: ChainId
    frozen_height: Height
    latest_height: Height
    max_clock_drift: Duration
    trust_level: TrustLevel
    trusting_period: Duration
    unbonding_period: Duration
    upgrade_path: list[str]


@dataclass
@cmd("query client state")
class QueryClientState(Cmd[ClientState]):
    chain_id: ChainId
    client_id: ClientId
    height: Optional[int] = None
    proof: bool = False

    def args(self) -> List[str]:
        args = []

        if self.height is not None:
            args.extend(['--height', str(self.height)])
        if self.proof:
            args.append('--proof')

        args.extend([self.chain_id, self.client_id])

        return args

    def process(self, result: Any) -> ClientState:
        return from_dict(ClientState, result[0])


# -----------------------------------------------------------------------------

@dataclass
class TxConnInitRes:
    connection_id: ConnectionId


@cmd("tx raw conn-init")
@dataclass
class TxConnInit(Cmd[TxConnInitRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    src_client_id: ClientId
    dst_client_id: ClientId

    def args(self) -> List[str]:
        return [self.dst_chain_id, self.src_chain_id,
                self.dst_client_id, self.src_client_id,
                "default-conn",
                "default-conn"]

    def process(self, result: Any) -> TxConnInitRes:
        return from_dict(TxConnInitRes, result[0]['OpenInitConnection'])


# -----------------------------------------------------------------------------

@dataclass
class TxConnTryRes:
    connection_id: ConnectionId


@cmd("tx raw conn-try")
@dataclass
class TxConnTry(Cmd[TxConnTryRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    src_client_id: ClientId
    dst_client_id: ClientId
    src_conn_id: ConnectionId

    def args(self) -> List[str]:
        return [self.dst_chain_id, self.src_chain_id,
                self.dst_client_id, self.src_client_id,
                "default-conn", self.src_conn_id]

    def process(self, result: Any) -> TxConnTryRes:
        return from_dict(TxConnTryRes, result[0]['OpenTryConnection'])


# -----------------------------------------------------------------------------

@dataclass
class TxConnAckRes:
    connection_id: ConnectionId


@cmd("tx raw conn-ack")
@dataclass
class TxConnAck(Cmd[TxConnAckRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    src_client_id: ClientId
    dst_client_id: ClientId
    src_conn_id: ConnectionId
    dst_conn_id: ConnectionId

    def args(self) -> List[str]:
        return [self.dst_chain_id, self.src_chain_id,
                self.dst_client_id, self.src_client_id,
                self.dst_conn_id, self.src_conn_id]

    def process(self, result: Any) -> TxConnAckRes:
        return from_dict(TxConnAckRes, result[0]['OpenAckConnection'])


# -----------------------------------------------------------------------------

@dataclass
class TxConnConfirmRes:
    connection_id: ConnectionId


@cmd("tx raw conn-confirm")
@dataclass
class TxConnConfirm(Cmd[TxConnConfirmRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    src_client_id: ClientId
    dst_client_id: ClientId
    src_conn_id: ConnectionId
    dst_conn_id: ConnectionId

    def args(self) -> List[str]:
        return [self.dst_chain_id, self.src_chain_id,
                self.dst_client_id, self.src_client_id,
                self.dst_conn_id, self.src_conn_id]

    def process(self, result: Any) -> TxConnConfirmRes:
        return from_dict(TxConnConfirmRes, result[0]['OpenConfirmConnection'])


# -----------------------------------------------------------------------------

@dataclass
class Version:
    features: list[str]
    identifier: str


@dataclass
class Counterparty:
    client_id: ClientId
    connection_id: ConnectionId
    prefix: list[int]


@dataclass
class ConnectionEnd:
    client_id: ClientId
    counterparty: Counterparty
    delay_period: int
    state: str
    versions: list[Version]


@cmd("query connection end")
@dataclass
class QueryConnectionEnd(Cmd[ConnectionEnd]):
    chain_id: ChainId
    connection_id: ConnectionId

    def args(self) -> List[str]:
        return [self.chain_id, self.connection_id]

    def process(self, result: Any) -> ConnectionEnd:
        return from_dict(ConnectionEnd, result[0])

# -----------------------------------------------------------------------------


@dataclass
class TxChanOpenInitRes:
    channel_id: ChannelId
    connection_id: ConnectionId
    counterparty_channel_id: Optional[ChannelId]
    counterparty_port_id: PortId
    height: BlockHeight
    port_id: PortId


@cmd("tx raw chan-open-init")
@dataclass
class TxChanOpenInit(Cmd[TxChanOpenInitRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    connection_id: ConnectionId
    dst_port_id: PortId
    src_port_id: PortId
    ordering: Optional[Ordering] = None

    def args(self) -> List[str]:
        args = [self.dst_chain_id, self.src_chain_id,
                self.connection_id,
                self.dst_port_id, self.src_port_id,
                "defaultChannel", "defaultChannel"]

        if self.ordering is not None:
            args.extend(['--ordering', str(self.ordering)])

        return args

    def process(self, result: Any) -> TxChanOpenInitRes:
        return from_dict(TxChanOpenInitRes, result[0]['OpenInitChannel'])

# -----------------------------------------------------------------------------


@dataclass
class TxChanOpenTryRes:
    channel_id: ChannelId
    connection_id: ConnectionId
    counterparty_channel_id: ChannelId
    counterparty_port_id: ChannelId
    height: BlockHeight
    port_id: PortId


@cmd("tx raw chan-open-try")
@dataclass
class TxChanOpenTry(Cmd[TxChanOpenTryRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    connection_id: ConnectionId
    dst_port_id: PortId
    src_port_id: PortId
    src_channel_id: ChannelId
    ordering: Optional[Ordering] = None

    def args(self) -> List[str]:
        args = [self.dst_chain_id, self.src_chain_id,
                self.connection_id,
                self.dst_port_id, self.src_port_id,
                "defaultChannel", self.src_channel_id]

        if self.ordering is not None:
            args.extend(['--ordering', str(self.ordering)])

        return args

    def process(self, result: Any) -> TxChanOpenTryRes:
        return from_dict(TxChanOpenTryRes, result[0]['OpenTryChannel'])

# -----------------------------------------------------------------------------


@dataclass
class TxChanOpenAckRes:
    channel_id: ChannelId
    connection_id: ConnectionId
    counterparty_channel_id: ChannelId
    counterparty_port_id: ChannelId
    height: BlockHeight
    port_id: PortId


@cmd("tx raw chan-open-ack")
@dataclass
class TxChanOpenAck(Cmd[TxChanOpenAckRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    connection_id: ConnectionId
    dst_port_id: PortId
    src_port_id: PortId
    dst_channel_id: ChannelId
    src_channel_id: ChannelId

    def args(self) -> List[str]:
        args = [self.dst_chain_id, self.src_chain_id,
                self.connection_id,
                self.dst_port_id, self.src_port_id,
                self.dst_channel_id, self.src_channel_id]

        return args

    def process(self, result: Any) -> TxChanOpenAckRes:
        return from_dict(TxChanOpenAckRes, result[0]['OpenAckChannel'])

# -----------------------------------------------------------------------------


@dataclass
class TxChanOpenConfirmRes:
    channel_id: ChannelId
    connection_id: ConnectionId
    counterparty_channel_id: ChannelId
    counterparty_port_id: ChannelId
    height: BlockHeight
    port_id: PortId


@cmd("tx raw chan-open-confirm")
@dataclass
class TxChanOpenConfirm(Cmd[TxChanOpenConfirmRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    connection_id: ConnectionId
    dst_port_id: PortId
    src_port_id: PortId
    dst_channel_id: ChannelId
    src_channel_id: ChannelId

    def args(self) -> List[str]:
        args = [self.dst_chain_id, self.src_chain_id,
                self.connection_id,
                self.dst_port_id, self.src_port_id,
                self.dst_channel_id, self.src_channel_id]

        return args

    def process(self, result: Any) -> TxChanOpenConfirmRes:
        return from_dict(TxChanOpenConfirmRes, result[0]['OpenConfirmChannel'])

# -----------------------------------------------------------------------------


@dataclass
class Remote:
    channel_id: ChannelId
    port_id: PortId


@dataclass
class ChannelEnd:
    connection_hops: list[Any]
    ordering: str
    remote: Remote
    state: str
    version: str


@cmd("query channel end")
@dataclass
class QueryChannelEnd(Cmd[ChannelEnd]):
    chain_id: ChainId
    connection_id: ConnectionId
    channel_id: ChannelId

    def args(self) -> List[str]:
        return [self.chain_id, self.connection_id, self.channel_id]

    def process(self, result: Any) -> ChannelEnd:
        return from_dict(ChannelEnd, result[0])

# -----------------------------------------------------------------------------


def split():
    sleep(0.5)
    print()


# CLIENT creation and manipulation
# =============================================================================

def create_client(c, dst: ChainId, src: ChainId) -> ClientCreated:
    cmd = TxCreateClient(dst_chain_id=dst, src_chain_id=src)
    client = cmd.run(c).success()
    l.info(f'Created client: {client.client_id}')
    return client


def update_client(c, dst: ChainId, src: ChainId, client_id: ClientId) -> ClientUpdated:
    cmd = TxUpdateClient(dst_chain_id=dst, src_chain_id=src,
                         dst_client_id=client_id)
    res = cmd.run(c).success()
    l.info(f'Updated client to: {res.consensus_height}')
    return res


def query_client_state(c, chain_id: ChainId, client_id: ClientId) -> Tuple[ClientId, ClientState]:
    cmd = QueryClientState(chain_id, client_id)
    res = cmd.run(c).success()
    l.debug(f'State of client {client_id} is: {res}')
    return client_id, res


def create_update_query_client(c, dst: ChainId, src: ChainId) -> ClientId:
    client = create_client(c, dst, src)
    split()
    query_client_state(c, dst, client.client_id)
    split()
    update_client(c, dst, src, client.client_id)
    split()
    query_client_state(c, dst, client.client_id)
    split()
    return client.client_id


# CONNECTION handshake
# =============================================================================

def conn_init(c,
              src: ChainId, dst: ChainId,
              src_client: ClientId, dst_client: ClientId
              ) -> ConnectionId:

    cmd = TxConnInit(src_chain_id=src, dst_chain_id=dst,
                     src_client_id=src_client, dst_client_id=dst_client)
    res = cmd.run(c).success()
    l.info(
        f'ConnOpen init submitted to {dst} and obtained connection id {res.connection_id}')
    return res.connection_id


def conn_try(c,
             src: ChainId, dst: ChainId,
             src_client: ClientId, dst_client: ClientId,
             src_conn: ConnectionId
             ) -> ConnectionId:

    cmd = TxConnTry(src_chain_id=src, dst_chain_id=dst, src_client_id=src_client, dst_client_id=dst_client,
                    src_conn_id=src_conn)
    res = cmd.run(c).success()
    l.info(
        f'ConnOpen try submitted to {dst} and obtained connection id {res.connection_id}')
    return res.connection_id


def conn_ack(c,
             src: ChainId, dst: ChainId,
             src_client: ClientId, dst_client: ClientId,
             src_conn: ConnectionId, dst_conn: ConnectionId
             ) -> ConnectionId:

    cmd = TxConnAck(src_chain_id=src, dst_chain_id=dst, src_client_id=src_client, dst_client_id=dst_client,
                    src_conn_id=src_conn, dst_conn_id=dst_conn)
    res = cmd.run(c).success()
    l.info(
        f'ConnOpen ack submitted to {dst} and obtained connection id {res.connection_id}')
    return res.connection_id


def conn_confirm(c,
                 src: ChainId, dst: ChainId,
                 src_client: ClientId, dst_client: ClientId,
                 src_conn: ConnectionId, dst_conn: ConnectionId
                 ) -> ConnectionId:

    cmd = TxConnConfirm(src_chain_id=src, dst_chain_id=dst, src_client_id=src_client, dst_client_id=dst_client,
                        src_conn_id=src_conn, dst_conn_id=dst_conn)
    res = cmd.run(c).success()
    l.info(
        f'ConnOpen confirm submitted to {dst} and obtained connection id {res.connection_id}')
    return res.connection_id


def connection_handshake(c,
                         side_a: ChainId, side_b: ChainId,
                         client_a: ClientId, client_b: ClientId
                         ) -> Tuple[ConnectionId, ConnectionId]:

    a_conn_id = conn_init(c, side_a, side_b, client_a, client_b)
    split()
    b_conn_id = conn_try(c, side_b, side_a, client_b, client_a, a_conn_id)
    split()
    ack_res = conn_ack(c, side_a, side_b, client_a,
                       client_b, b_conn_id, a_conn_id)
    if ack_res != a_conn_id:
        l.error(
            f'Incorrect connection id returned from conn ack: expected=({a_conn_id})/got=({ack_res})')
        exit(1)

    split()

    confirm_res = conn_confirm(
        c, side_b, side_a, client_b, client_a, a_conn_id, b_conn_id)

    if confirm_res != b_conn_id:
        l.error(
            f'Incorrect connection id returned from conn confirm: expected=({b_conn_id})/got=({confirm_res})')
        exit(1)

    a_conn_end = query_connection_end(c, side_a, a_conn_id)
    if a_conn_end.state != 'Open':
        l.error(
            f'Connection end with id {a_conn_id} is not in Open state, got: {a_conn_end.state}')
        exit(1)

    b_conn_end = query_connection_end(c, side_b, b_conn_id)
    if b_conn_end.state != 'Open':
        l.error(
            f'Connection end with id {b_conn_id} is not in Open state, got: {b_conn_end.state}')
        exit(1)

    return (a_conn_id, b_conn_id)


# CONNECTION END query
# =============================================================================


def query_connection_end(c, chain_id: ChainId, conn_id: ConnectionId) -> ConnectionEnd:
    cmd = QueryConnectionEnd(chain_id, conn_id)
    res = cmd.run(c).success()

    l.debug(f'Status of connection end {conn_id}: {res}')

    return res


# CHANNEL handshake
# =============================================================================


def chan_open_init(c,
                   src: ChainId, dst: ChainId,
                   dst_conn: ConnectionId,
                   src_port: PortId = PortId('transfer'),
                   dst_port: PortId = PortId('transfer'),
                   ordering: Optional[Ordering] = None
                   ) -> ChannelId:

    cmd = TxChanOpenInit(src_chain_id=src, dst_chain_id=dst,
                         connection_id=dst_conn,
                         dst_port_id=dst_port, src_port_id=src_port,
                         ordering=ordering)

    res = cmd.run(c).success()
    l.info(
        f'ChanOpenInit submitted to {dst} and obtained channel id {res.channel_id}')
    return res.channel_id


def chan_open_try(c,
                  src: ChainId, dst: ChainId,
                  dst_conn: ConnectionId,
                  src_chan: ChannelId,
                  src_port: PortId = PortId('transfer'),
                  dst_port: PortId = PortId('transfer'),
                  ordering: Optional[Ordering] = None
                  ) -> ChannelId:

    cmd = TxChanOpenTry(src_chain_id=src, dst_chain_id=dst,
                        connection_id=dst_conn,
                        dst_port_id=dst_port, src_port_id=src_port,
                        src_channel_id=src_chan,
                        ordering=ordering)

    res = cmd.run(c).success()
    l.info(
        f'ChanOpenTry submitted to {dst} and obtained channel id {res.channel_id}')
    return res.channel_id


def chan_open_ack(c,
                  src: ChainId, dst: ChainId,
                  dst_conn: ConnectionId,
                  src_chan: ChannelId,
                  dst_chan: ChannelId,
                  src_port: PortId = PortId('transfer'),
                  dst_port: PortId = PortId('transfer'),
                  ) -> ChannelId:

    cmd = TxChanOpenAck(src_chain_id=src, dst_chain_id=dst,
                        connection_id=dst_conn,
                        dst_port_id=dst_port, src_port_id=src_port,
                        dst_channel_id=dst_chan,
                        src_channel_id=src_chan)

    res = cmd.run(c).success()
    l.info(
        f'ChanOpenAck submitted to {dst} and got channel id {res.channel_id}')
    return res.channel_id


def chan_open_confirm(c,
                      src: ChainId, dst: ChainId,
                      dst_conn: ConnectionId,
                      src_chan: ChannelId,
                      dst_chan: ChannelId,
                      src_port: PortId = PortId('transfer'),
                      dst_port: PortId = PortId('transfer'),
                      ) -> ChannelId:

    cmd = TxChanOpenConfirm(src_chain_id=src, dst_chain_id=dst,
                            connection_id=dst_conn,
                            dst_port_id=dst_port, src_port_id=src_port,
                            dst_channel_id=dst_chan,
                            src_channel_id=src_chan)

    res = cmd.run(c).success()
    l.info(
        f'ChanOpenConfirm submitted to {dst} and got channel id {res.channel_id}')
    return res.channel_id


def channel_handshake(c,
                      side_a: ChainId, side_b: ChainId,
                      conn_a: ConnectionId, conn_b: ConnectionId,
                      ) -> Tuple[ChannelId, ChannelId]:

    a_chan_id = chan_open_init(c, dst=side_a, src=side_b, dst_conn=conn_a)

    split()

    b_chan_id = chan_open_try(
        c, dst=side_b, src=side_a, dst_conn=conn_b, src_chan=a_chan_id)

    split()

    ack_res = chan_open_ack(c, dst=side_a, src=side_b,
                            dst_conn=conn_a, dst_chan=a_chan_id, src_chan=b_chan_id)

    if ack_res != a_chan_id:
        l.error(
            f'Incorrect channel id returned from chan open ack: expected={a_chan_id} got={ack_res}')
        exit(1)

    confirm_res = chan_open_confirm(
        c, dst=side_b, src=side_a, dst_conn=conn_b, dst_chan=b_chan_id, src_chan=a_chan_id)

    if confirm_res != b_chan_id:
        l.error(
            f'Incorrect channel id returned from chan open confirm: expected={b_chan_id} got={confirm_res}')
        exit(1)

    split()

    a_chan_end = query_channel_end(c, side_a, conn_a, a_chan_id)
    if a_chan_end.state != 'Open':
        l.warn(
            f'Channel end with id {a_chan_id} on chain {side_a} is not in Open state, got: {a_chan_end.state}')
        # exit(1)

    b_chan_end = query_channel_end(c, side_b, conn_b, a_chan_id)
    if b_chan_end.state != 'Open':
        l.warn(
            f'Channel end with id {b_chan_id} on chain {side_b} is not in Open state, got: {b_chan_end.state}')
        # exit(1)

    return a_chan_id, b_chan_id

# CHANNEL END query
# =============================================================================


def query_channel_end(c, chain_id: ChainId, conn_id: ConnectionId, chan_id: ChannelId) -> ChannelEnd:
    cmd = QueryChannelEnd(chain_id, conn_id, chan_id)
    res = cmd.run(c).success()

    l.debug(f'Status of channel end {chan_id}: {res}')

    return res

# =============================================================================


# todo(romac): kindly move the code below to its rightful place
# -----------------------------------------------------------------------------


@dataclass
class TxPacketSendRes:
    sequence: Sequence


@cmd("tx raw packet-send")
@dataclass
class TxPacketSend(Cmd[TxPacketSendRes]):
    src_chain_id: str
    dst_chain_id: str
    src_port: PortId
    src_channel: ChannelId

    def args(self) -> List[str]:
        return [self.src_chain_id, self.dst_chain_id, self.src_port, self.src_channel, "9999", "1000"]

    def process(self, result: Any) -> TxPacketSendRes:
        return from_dict(TxPacketSendRes, result[0][0]['SendPacketChannel']['packet'])

# -----------------------------------------------------------------------------


@dataclass
class TxPacketRecvRes:
    sequence: Sequence


@cmd("tx raw packet-recv")
@dataclass
class TxPacketRecv(Cmd[TxPacketRecvRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    src_port: PortId
    src_channel: ChannelId

    def args(self) -> List[str]:
        return [self.dst_chain_id, self.src_chain_id, self.src_port, self.src_channel]

    # todo: the output here is a list of [`UpdateClient`,`WriteAcknowledgementChannel`]
    #   possible that the UpdateClient is missing, and then `WriteAcknowledgementChannel` will no longer be at index 1
    #   so we might need a more sophisticated way to extract this event...
    def process(self, result: Any) -> TxPacketRecvRes:
        return from_dict(TxPacketRecvRes, result[0][1]['WriteAcknowledgementChannel']['packet'])

# -----------------------------------------------------------------------------


@dataclass
class TxPacketAckRes:
    sequence: Sequence


@cmd("tx raw packet-ack")
@dataclass
class TxPacketAck(Cmd[TxPacketAckRes]):
    src_chain_id: ChainId
    dst_chain_id: ChainId
    src_port: PortId
    src_channel: ChannelId

    def args(self) -> List[str]:
        return [self.dst_chain_id, self.src_chain_id, self.src_port, self.src_channel]

    # todo: same as for TxPacketRecv:
    #   the output here is a list of [`UpdateClient`,`AcknowledgePacketChannel`]
    #   possible that the UpdateClient is missing, and then `WriteAcknowledgementChannel` will no longer be at index 1
    #   so we might need a more sophisticated way to extract this event...
    def process(self, result: Any) -> TxPacketAckRes:
        return from_dict(TxPacketAckRes, result[0][1]['AcknowledgePacketChannel']['packet'])

# -----------------------------------------------------------------------------


# TRANSFER (packet send)
# =============================================================================

def packet_send(c, src: ChainId, dst: ChainId, src_port: PortId, src_channel: ChannelId) -> Sequence:
    cmd = TxPacketSend(src_chain_id=src, dst_chain_id=dst,
                       src_port=src_port, src_channel=src_channel)

    res = cmd.run(c).success()
    l.info(f'PacketSend to {src} and obtained sequence number {res.sequence}')
    return res.sequence


def packet_recv(c, dst: ChainId, src: ChainId, src_port: PortId, src_channel: ChannelId) -> Sequence:
    cmd = TxPacketRecv(src_chain_id=src, dst_chain_id=dst,
                       src_port=src_port, src_channel=src_channel)

    res = cmd.run(c).success()
    l.info(f'PacketRecv to {dst} done for sequence number {res.sequence}')
    return res.sequence


def packet_ack(c, dst: ChainId, src: ChainId, src_port: PortId, src_channel: ChannelId) -> Sequence:
    cmd = TxPacketAck(src_chain_id=src, dst_chain_id=dst,
                      src_port=src_port, src_channel=src_channel)

    res = cmd.run(c).success()
    l.info(f'PacketAck to {dst} done for sequence number {res.sequence}')
    return res.sequence


def packet_ping_pong(c,
                     side_a: ChainId, side_b: ChainId,
                     a_chan: ChannelId, b_chan: ChannelId,
                     port_id: PortId = PortId('transfer')):

    seq_send_a = packet_send(c, side_a, side_b, port_id, a_chan)

    split()

    seq_recv_a = packet_recv(c, side_b, side_a, port_id, b_chan)

    if seq_send_a != seq_recv_a:
        l.error(
            f'Mismatched sequence numbers for path {side_a} -> {side_b} : Sent={seq_send_a} versus Received={seq_recv_a}')

    split()

    # write the ack
    seq_ack_a = packet_ack(c, side_a, side_b, port_id, a_chan)
    if seq_recv_a != seq_ack_a:
        l.error(
            f'Mismatched sequence numbers for ack on path {side_a} -> {side_b} : Recv={seq_recv_a} versus Ack={seq_ack_a}')

    split()

    seq_send_b = packet_send(c, side_b, side_a, port_id, b_chan)

    split()

    seq_recv_b = packet_recv(c, side_a, side_b, port_id, a_chan)

    if seq_send_b != seq_recv_b:
        l.error(
            f'Mismatched sequence numbers for path {side_b} -> {side_b} : Sent={seq_send_b} versus Received={seq_recv_b}')

    split()

    seq_ack_b = packet_ack(c, side_b, side_a, port_id, a_chan)

    if seq_recv_b != seq_ack_b:
        l.error(
            f'Mismatched sequence numbers for ack on path {side_a} -> {side_b} : Recv={seq_recv_b} versus Ack={seq_ack_b}')


def run(c: Path):
    IBC_0 = ChainId('ibc-0')
    IBC_1 = ChainId('ibc-1')

    ibc0_client_id = create_update_query_client(c, IBC_0, IBC_1)
    ibc1_client_id = create_update_query_client(c, IBC_1, IBC_0)

    split()

    ibc0_conn_id, ibc1_conn_id = connection_handshake(
        c, IBC_1, IBC_0, ibc1_client_id, ibc0_client_id)

    split()

    ibc1_chan_id, ibc0_chan_id = channel_handshake(
        c, IBC_1, IBC_0, ibc1_conn_id, ibc0_conn_id)

    split()

    packet_ping_pong(c, IBC_0, IBC_1, ibc0_chan_id, ibc1_chan_id)


def main():
    l.basicConfig(
        level=l.DEBUG,
        format='[%(asctime)s] [%(levelname)8s] %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S')

    parser = argparse.ArgumentParser(
        description='Test all relayer commands, end-to-end')

    parser.add_argument('-c', '--config',
                        help='configuration file for the relayer',
                        metavar='CONFIG_FILE',
                        required=True,
                        type=Path)

    args = parser.parse_args()

    if not args.config.exists():
        print(
            f'error: supplied configuration file does not exist: {args.config}')
        exit(1)

    run(args.config)


if __name__ == "__main__":
    main()
