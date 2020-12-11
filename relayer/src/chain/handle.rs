use std::sync::Arc;

use crossbeam_channel as channel;

use dyn_clone::DynClone;

use ibc_proto::ibc::core::channel::v1::{
    PacketAckCommitment, QueryPacketCommitmentsRequest, QueryUnreceivedPacketsRequest,
};

use ibc::{
    events::IBCEvent,
    ics02_client::client_def::{AnyClientState, AnyConsensusState, AnyHeader},
    ics03_connection::connection::ConnectionEnd,
    ics04_channel::channel::{ChannelEnd, QueryPacketEventDataRequest},
    ics24_host::identifier::{ChannelId, ConnectionId, PortId},
    proofs::Proofs,
};
use ibc::{ics23_commitment::commitment::CommitmentPrefix, Height};
use ibc::{
    ics23_commitment::merkle::MerkleProof,
    ics24_host::{identifier::ChainId, identifier::ClientId, Path},
};

// FIXME: the handle should not depend on tendermint-specific types
use tendermint::account::Id as AccountId;

use super::QueryResponse;

use crate::connection::ConnectionMsgType;
use crate::keyring::store::KeyEntry;
use crate::{error::Error, event::monitor::EventBatch};

mod prod;

pub use prod::ProdChainHandle;
use std::fmt::Debug;

pub type Subscription = channel::Receiver<Arc<EventBatch>>;

pub type ReplyTo<T> = channel::Sender<Result<T, Error>>;
pub type Reply<T> = channel::Receiver<Result<T, Error>>;

pub fn reply_channel<T>() -> (ReplyTo<T>, Reply<T>) {
    channel::bounded(1)
}

/// Inputs that a Handle may send to a Runtime.
#[derive(Clone, Debug)]
pub enum HandleInput {
    Terminate {
        reply_to: ReplyTo<()>,
    },

    Subscribe {
        reply_to: ReplyTo<Subscription>,
    },

    Query {
        path: Path,
        height: Height,
        prove: bool,
        reply_to: ReplyTo<QueryResponse>,
    },

    SendMsgs {
        proto_msgs: Vec<prost_types::Any>,
        reply_to: ReplyTo<Vec<String>>,
    },

    // GetHeader {
    //     height: Height,
    //     reply_to: ReplyTo<AnyHeader>,
    // },
    GetMinimalSet {
        from: Height,
        to: Height,
        reply_to: ReplyTo<Vec<AnyHeader>>,
    },

    // Submit {
    //     transaction: EncodedTransaction,
    //     reply_to: ReplyTo<()>,
    // },
    Signer {
        reply_to: ReplyTo<AccountId>,
    },

    Key {
        reply_to: ReplyTo<KeyEntry>,
    },

    ModuleVersion {
        port_id: PortId,
        reply_to: ReplyTo<String>,
    },

    QueryLatestHeight {
        reply_to: ReplyTo<Height>,
    },

    // CreatePacket {
    //     event: IBCEvent,
    //     reply_to: ReplyTo<Packet>,
    // },
    BuildHeader {
        trusted_height: Height,
        target_height: Height,
        reply_to: ReplyTo<AnyHeader>,
    },

    BuildClientState {
        height: Height,
        reply_to: ReplyTo<AnyClientState>,
    },

    BuildConsensusState {
        height: Height,
        reply_to: ReplyTo<AnyConsensusState>,
    },

    BuildConnectionProofsAndClientState {
        message_type: ConnectionMsgType,
        connection_id: ConnectionId,
        client_id: ClientId,
        height: Height,
        reply_to: ReplyTo<(Option<AnyClientState>, Proofs)>,
    },

    QueryClientState {
        client_id: ClientId,
        height: Height,
        reply_to: ReplyTo<AnyClientState>,
    },

    QueryCommitmentPrefix {
        reply_to: ReplyTo<CommitmentPrefix>,
    },

    QueryCompatibleVersions {
        reply_to: ReplyTo<Vec<String>>,
    },

    QueryConnection {
        connection_id: ConnectionId,
        height: Height,
        reply_to: ReplyTo<ConnectionEnd>,
    },

    QueryChannel {
        port_id: PortId,
        channel_id: ChannelId,
        height: Height,
        reply_to: ReplyTo<ChannelEnd>,
    },

    ProvenClientState {
        client_id: ClientId,
        height: Height,
        reply_to: ReplyTo<(AnyClientState, MerkleProof)>,
    },

    ProvenConnection {
        connection_id: ConnectionId,
        height: Height,
        reply_to: ReplyTo<(ConnectionEnd, MerkleProof)>,
    },

    ProvenClientConsensus {
        client_id: ClientId,
        consensus_height: Height,
        height: Height,
        reply_to: ReplyTo<(AnyConsensusState, MerkleProof)>,
    },

    BuildChannelProofs {
        port_id: PortId,
        channel_id: ChannelId,
        height: Height,
        reply_to: ReplyTo<Proofs>,
    },

    ProvenPacketCommitment {
        port_id: PortId,
        channel_id: ChannelId,
        sequence: u64,
        height: Height,
        reply_to: ReplyTo<(Vec<u8>, MerkleProof)>,
    },

    QueryPacketCommitments {
        request: QueryPacketCommitmentsRequest,
        reply_to: ReplyTo<(Vec<PacketAckCommitment>, Height)>,
    },

    QueryUnreceivedPackets {
        request: QueryUnreceivedPacketsRequest,
        reply_to: ReplyTo<Vec<u64>>,
    },

    QueryPacketEventData {
        request: QueryPacketEventDataRequest,
        reply_to: ReplyTo<Vec<IBCEvent>>,
    },
}

// Make `clone` accessible to a ChainHandle object
dyn_clone::clone_trait_object!(ChainHandle);

pub trait ChainHandle: DynClone + Send + Sync + Debug {
    fn id(&self) -> ChainId;

    fn query(&self, path: Path, height: Height, prove: bool) -> Result<QueryResponse, Error>;

    fn subscribe(&self, chain_id: ChainId) -> Result<Subscription, Error>;

    /// Send a transaction with `msgs` to chain.
    fn send_msgs(&self, proto_msgs: Vec<prost_types::Any>) -> Result<Vec<String>, Error>;

    fn get_minimal_set(&self, from: Height, to: Height) -> Result<Vec<AnyHeader>, Error>;

    fn get_signer(&self) -> Result<AccountId, Error>;

    fn get_key(&self) -> Result<KeyEntry, Error>;

    fn module_version(&self, port_id: &PortId) -> Result<String, Error>;

    fn query_latest_height(&self) -> Result<Height, Error>;

    fn query_client_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<AnyClientState, Error>;

    fn query_commitment_prefix(&self) -> Result<CommitmentPrefix, Error>;

    fn query_compatible_versions(&self) -> Result<Vec<String>, Error>;

    fn query_connection(
        &self,
        connection_id: &ConnectionId,
        height: Height,
    ) -> Result<ConnectionEnd, Error>;

    fn query_channel(
        &self,
        port_id: &PortId,
        channel_id: &ChannelId,
        height: Height,
    ) -> Result<ChannelEnd, Error>;

    fn proven_client_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<(AnyClientState, MerkleProof), Error>;

    fn proven_connection(
        &self,
        connection_id: &ConnectionId,
        height: Height,
    ) -> Result<(ConnectionEnd, MerkleProof), Error>;

    fn proven_client_consensus(
        &self,
        client_id: &ClientId,
        consensus_height: Height,
        height: Height,
    ) -> Result<(AnyConsensusState, MerkleProof), Error>;

    fn build_header(
        &self,
        trusted_height: Height,
        target_height: Height,
    ) -> Result<AnyHeader, Error>;

    /// Constructs a client state at the given height
    fn build_client_state(&self, height: Height) -> Result<AnyClientState, Error>;

    /// Constructs a consensus state at the given height
    fn build_consensus_state(&self, height: Height) -> Result<AnyConsensusState, Error>;

    fn build_connection_proofs_and_client_state(
        &self,
        message_type: ConnectionMsgType,
        connection_id: &ConnectionId,
        client_id: &ClientId,
        height: Height,
    ) -> Result<(Option<AnyClientState>, Proofs), Error>;

    fn build_channel_proofs(
        &self,
        port_id: &PortId,
        channel_id: &ChannelId,
        height: Height,
    ) -> Result<Proofs, Error>;

    fn proven_packet_commitment(
        &self,
        port_id: &PortId,
        channel_id: &ChannelId,
        sequence: u64,
        height: Height,
    ) -> Result<(Vec<u8>, MerkleProof), Error>;

    fn query_packet_commitments(
        &self,
        request: QueryPacketCommitmentsRequest,
    ) -> Result<(Vec<PacketAckCommitment>, Height), Error>;

    fn query_unreceived_packets(
        &self,
        request: QueryUnreceivedPacketsRequest,
    ) -> Result<Vec<u64>, Error>;

    fn query_txs(&self, request: QueryPacketEventDataRequest) -> Result<Vec<IBCEvent>, Error>;
}
