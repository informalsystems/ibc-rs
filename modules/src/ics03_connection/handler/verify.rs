use crate::ics02_client::client_def::AnyClientState;
use crate::ics02_client::state::{ClientState, ConsensusState};
use crate::ics03_connection::connection::ConnectionEnd;
use crate::ics03_connection::context::ConnectionReader;
use crate::ics03_connection::error::{Error, Kind};
use crate::ics23_commitment::commitment::CommitmentProof;
use crate::ics24_host::identifier::ConnectionId;
use crate::proofs::{ConsensusProof, Proofs};
use tendermint::block::Height;

pub fn verify_proofs(
    ctx: &dyn ConnectionReader,
    id: &ConnectionId,
    client_state: Option<AnyClientState>,
    connection_end: &ConnectionEnd,
    expected_conn: &ConnectionEnd,
    proofs: &Proofs,
) -> Result<(), Error> {
    verify_connection_proof(
        ctx,
        id,
        connection_end,
        expected_conn,
        proofs.height(),
        proofs.object_proof(),
    )?;

    // If a client proof is present verify it.
    match client_state {
        None => (),
        Some(state) => verify_client_proof(
            ctx,
            connection_end,
            state,
            proofs.height(),
            proofs
                .client_proof()
                .as_ref()
                .ok_or_else(|| Kind::NullClientProof)?,
        )?,
    };

    // If a consensus proof is present verify it.
    match proofs.consensus_proof() {
        None => Ok(()),
        Some(proof) => Ok(verify_consensus_proof(
            ctx,
            connection_end,
            proofs.height(),
            &proof,
        )?),
    }
}

pub fn verify_connection_proof(
    ctx: &dyn ConnectionReader,
    id: &ConnectionId,
    connection_end: &ConnectionEnd,
    expected_conn: &ConnectionEnd,
    proof_height: Height,
    proof: &CommitmentProof,
) -> Result<(), Error> {
    // Fetch the client state (IBC client on the local chain).
    let client = ctx
        .fetch_client_state(connection_end.client_id())
        .ok_or_else(|| Kind::MissingClient(connection_end.client_id().clone()))?;
    if client.is_frozen() {
        return Err(Kind::FrozenClient
            .context(connection_end.client_id().to_string())
            .into());
    }

    // Verify the proof for the connection state against the expected connection end.
    Ok(client
        .verify_connection_state(
            proof_height,
            connection_end.counterparty().prefix(),
            proof,
            connection_end.counterparty().connection_id(),
            expected_conn,
        )
        .map_err(|_| Kind::InvalidProof.context(id.to_string()))?)
}

pub fn verify_client_proof(
    ctx: &dyn ConnectionReader,
    connection_end: &ConnectionEnd,
    client_state: AnyClientState,
    proof_height: Height,
    proof: &CommitmentProof,
) -> Result<(), Error> {
    let client = ctx
        .fetch_client_state(connection_end.client_id())
        .ok_or_else(|| Kind::MissingClient(connection_end.client_id().clone()))?;

    let consensus_state = ctx
        .fetch_client_consensus_state(connection_end.client_id(), proof_height)
        .ok_or_else(|| {
            Kind::MissingClientConsensusState.context(connection_end.client_id().to_string())
        })?;

    Ok(client
        .verify_client_full_state(
            proof_height,
            consensus_state.root(),
            connection_end.counterparty().prefix(),
            connection_end.counterparty().client_id(),
            proof,
            &client_state,
        )
        .map_err(|_| {
            Kind::ClientStateVerificationFailure.context(connection_end.client_id().to_string())
        })?)
}

pub fn verify_consensus_proof(
    ctx: &dyn ConnectionReader,
    connection_end: &ConnectionEnd,
    proof_height: Height,
    proof: &ConsensusProof,
) -> Result<(), Error> {
    // Fetch the client state (IBC client on the local chain).
    let client = ctx
        .fetch_client_state(connection_end.client_id())
        .ok_or_else(|| Kind::MissingClient(connection_end.client_id().clone()))?;

    if client.is_frozen() {
        return Err(Kind::FrozenClient
            .context(connection_end.client_id().to_string())
            .into());
    }

    // Fetch the expected consensus state from the historical (local) header data.
    let expected_consensus = ctx
        .fetch_self_consensus_state(proof.height())
        .ok_or_else(|| Kind::MissingLocalConsensusState.context(proof.height().to_string()))?;

    Ok(client
        .verify_client_consensus_state(
            proof_height,
            connection_end.counterparty().prefix(),
            proof.proof(),
            connection_end.counterparty().client_id(),
            proof.height(),
            &expected_consensus,
        )
        .map_err(|_| Kind::ConsensusStateVerificationFailure.context(proof.height().to_string()))?)
}

pub fn check_client_consensus_height(
    ctx: &dyn ConnectionReader,
    claimed_height: Height,
) -> Result<(), Error> {
    // Fail if the consensus height is too advanced.
    if claimed_height > ctx.chain_current_height() {
        return Err(Kind::InvalidConsensusHeight
            .context(claimed_height.to_string())
            .into());
    }

    // Fail if the consensus height is too old (outside of trusting period).
    if claimed_height.value()
        < (ctx.chain_current_height().value() - ctx.chain_consensus_states_history_size() as u64)
    {
        return Err(Kind::StaleConsensusHeight
            .context(claimed_height.to_string())
            .into());
    }

    Ok(())
}
