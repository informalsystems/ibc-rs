use anomaly::{BoxError, Context};
use thiserror::Error;

use crate::ics24_host::identifier::{ClientId, ConnectionId};
use crate::Height;

pub type Error = anomaly::Error<Kind>;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum Kind {
    #[error("connection state unknown")]
    InvalidState(i32),

    #[error("connection exists (was initialized) already: {0}")]
    ConnectionExistsAlready(ConnectionId),

    #[error("a different connection exists (was initialized) already for the same connection identifier {0}")]
    ConnectionMismatch(ConnectionId),

    #[error("connection end for identifier {0} was never initialized")]
    UninitializedConnection(ConnectionId),

    #[error("consensus height claimed by the client on the other party is too advanced: {0} (host chain current height: {1})")]
    InvalidConsensusHeight(Height, Height),

    #[error("consensus height claimed by the client on the other party has been pruned: {0} (host chain oldest height: {1})")]
    StaleConsensusHeight(Height, Height),

    #[error("identifier error")]
    IdentifierError,

    #[error("ConnectionEnd domain object could not be constructed out of empty proto object")]
    EmptyProtoConnectionEnd,

    #[error("invalid version")]
    InvalidVersion,

    #[error("empty supported versions")]
    EmptyVersions,

    #[error("no common version")]
    NoCommonVersion,

    #[error("invalid address")]
    InvalidAddress,

    #[error("missing consensus proof height")]
    MissingProofHeight,

    #[error("missing consensus proof height")]
    MissingConsensusHeight,

    #[error("invalid connection proof")]
    InvalidProof,

    #[error("invalid signer")]
    InvalidSigner,

    #[error("no connection was found for the previous connection id provided {0}")]
    ConnectionNotFound(ConnectionId),

    #[error("invalid counterparty")]
    InvalidCounterparty,

    #[error("counterparty chosen connection id {0} is different than the connection id {1}")]
    ConnectionIdMismatch(ConnectionId, ConnectionId),

    #[error("missing counterparty")]
    MissingCounterparty,

    #[error("missing counterparty prefix")]
    MissingCounterpartyPrefix,

    #[error("the client id does not match any client state: {0}")]
    MissingClient(ClientId),

    #[error("client proof must be present")]
    NullClientProof,

    #[error("the client {0} running locally is frozen")]
    FrozenClient(ClientId),

    #[error("the connection proof verification failed")]
    ConnectionVerificationFailure,

    #[error("the consensus state at height {0} for client id {1} could not be retrieved")]
    MissingClientConsensusState(Height, ClientId),

    #[error("the local consensus state could not be retrieved")]
    MissingLocalConsensusState,

    #[error("the consensus proof verification failed (height: {0})")]
    ConsensusStateVerificationFailure(Height),

    #[error("the client state proof verification failed for client id: {0}")]
    ClientStateVerificationFailure(ClientId),
}

impl Kind {
    pub fn context(self, source: impl Into<BoxError>) -> Context<Self> {
        Context::new(self, Some(source.into()))
    }
}
