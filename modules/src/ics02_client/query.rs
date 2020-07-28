use std::marker::PhantomData;

use crate::ics23_commitment::{CommitmentPath, CommitmentProof};
use crate::ics03_connection::error::Kind;
use crate::ics24_host::identifier::ClientId;
use crate::path::{ClientStatePath, ConsensusStatePath};
use crate::try_from_raw::TryFromRaw;
use crate::Height;

//TODO: This might need to be migrated to ibc-proto crate. But ClientConnections (as array of strings)
// might not be part of an official proto file
#[derive(::prost::Message)]
pub struct RawClientConnections {
    #[prost(string, repeated, tag = "1")]
    pub connections: ::std::vec::Vec<String>,
}

impl TryFromRaw for Vec<String> {
    type RawType = RawClientConnections;
    type Error = anomaly::Error<Kind>;
    fn try_from(value: RawClientConnections) -> Result<Self, Self::Error> {
        if !value.connections.is_empty() {
            Ok(value.connections)
        } else {
            Err(Kind::ConnectionNotFound.into())
        }
    }
}

pub struct QueryClientFullState<CLS> {
    pub chain_height: Height,
    pub client_id: ClientId,
    pub client_state_path: ClientStatePath,
    pub prove: bool,
    marker: PhantomData<CLS>,
}

impl<CLS> QueryClientFullState<CLS> {
    pub fn new(chain_height: Height, client_id: ClientId, prove: bool) -> Self {
        Self {
            chain_height,
            client_id: client_id.clone(),
            client_state_path: ClientStatePath::new(client_id),
            prove,
            marker: PhantomData,
        }
    }
}

pub struct ClientFullStateResponse<CLS> {
    pub client_state: CLS,
    pub proof: Option<CommitmentProof>,
    pub proof_path: CommitmentPath,
    pub proof_height: Height,
}

impl<CLS> ClientFullStateResponse<CLS> {
    pub fn new(
        client_id: ClientId,
        client_state: CLS,
        abci_proof: Option<CommitmentProof>,
        proof_height: Height,
    ) -> Self {
        let proof_path = CommitmentPath::from_path(ClientStatePath::new(client_id));

        ClientFullStateResponse {
            client_state,
            proof: abci_proof,
            proof_path,
            proof_height,
        }
    }
}

pub struct QueryClientConsensusState<CS> {
    pub chain_height: Height,
    pub client_id: ClientId,
    pub consensus_height: Height,
    pub consensus_state_path: ConsensusStatePath,
    pub prove: bool,
    marker: PhantomData<CS>,
}

impl<CS> QueryClientConsensusState<CS> {
    pub fn new(
        chain_height: Height,
        client_id: ClientId,
        consensus_height: Height,
        prove: bool,
    ) -> Self {
        Self {
            chain_height,
            client_id: client_id.clone(),
            consensus_height,
            consensus_state_path: ConsensusStatePath::new(client_id, consensus_height),
            prove,
            marker: PhantomData,
        }
    }
}

pub struct ConsensusStateResponse<CS> {
    pub consensus_state: CS,
    pub proof: Option<CommitmentProof>,
    pub proof_path: CommitmentPath,
    pub proof_height: Height,
}

impl<CS> ConsensusStateResponse<CS> {
    pub fn new(
        client_id: ClientId,
        consensus_state: CS,
        abci_proof: Option<CommitmentProof>,
        proof_height: Height,
    ) -> Self {
        let proof_path =
            CommitmentPath::from_path(ConsensusStatePath::new(client_id, proof_height));

        ConsensusStateResponse {
            consensus_state,
            proof: abci_proof,
            proof_path,
            proof_height,
        }
    }
}
