use crate::ics03_connection::connection::{Counterparty, validate_versions, validate_version};
use crate::ics03_connection::error::{Kind, Error};
use crate::ics03_connection::exported::ConnectionCounterparty;
use crate::ics23_commitment::{CommitmentPrefix, CommitmentProof};
use crate::ics24_host::identifier::{ClientId, ConnectionId};
use crate::tx_msg::Msg;
use serde_derive::{Deserialize, Serialize};
use tendermint::account::Id as AccountId;
use anomaly::fail;

// TODO: Validate Proof for all Msgs

pub const TYPE_MSG_CONNECTION_OPEN_INIT: &str = "connection_open_init";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MsgConnectionOpenInit {
    connection_id: ConnectionId,
    client_id: ClientId,
    counterparty: Counterparty,
    signer: AccountId,
}

impl MsgConnectionOpenInit {
    pub fn new(
        connection_id: String,
        client_id: String,
        counterparty_connection_id: String,
        counterparty_client_id: String,
        counterparty_commitment_prefix: CommitmentPrefix,
        signer: AccountId,
    ) -> Result<MsgConnectionOpenInit, Error> {
        Ok(Self {
            connection_id: connection_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            client_id: client_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            counterparty: Counterparty::new(
                counterparty_client_id,
                counterparty_connection_id,
                counterparty_commitment_prefix,
            )
            .map_err(|e| Kind::IdentifierError.context(e))?,
            signer,
        })
    }
}

impl Msg for MsgConnectionOpenInit {
    type ValidationError = Error;

    fn route(&self) -> String {
        crate::keys::ROUTER_KEY.to_string()
    }

    fn get_type(&self) -> String {
        TYPE_MSG_CONNECTION_OPEN_INIT.to_string()
    }

    fn validate_basic(&self) -> Result<(), Self::ValidationError> {
        // All the validation is performed on creation
        self.counterparty.validate_basic()
    }

    fn get_sign_bytes(&self) -> Vec<u8> {
        todo!()
    }

    fn get_signers(&self) -> Vec<AccountId> {
        vec![self.signer]
    }
}

pub const TYPE_MSG_CONNECTION_OPEN_TRY: &str = "connection_open_try";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MsgConnectionOpenTry {
    connection_id: ConnectionId,
    client_id: ClientId,
    counterparty: Counterparty,
    counterparty_versions: Vec<String>,
    proof: ProofConnOpenTry,
    consensus_height: u64,
    signer: AccountId,
}

impl MsgConnectionOpenTry {
    pub fn new(
        connection_id: String,
        client_id: String,
        counterparty_connection_id: String,
        counterparty_client_id: String,
        counterparty_commitment_prefix: CommitmentPrefix,
        counterparty_versions: Vec<String>,
        proof_init: CommitmentProof,
        proof_consensus: CommitmentProof,
        proof_height: u64,
        consensus_height: u64,
        signer: AccountId,
    ) -> Result<MsgConnectionOpenTry, Error> {
        if proof_height == 0 || consensus_height == 0 {
            fail!(Kind::InvalidHeight, "Height cannot be zero");
        }

        Ok(Self {
            connection_id: connection_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            client_id: client_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            counterparty: Counterparty::new(
                counterparty_client_id,
                counterparty_connection_id,
                counterparty_commitment_prefix,
            ).map_err(|e| Kind::IdentifierError.context(e))?,
            counterparty_versions: validate_versions(counterparty_versions)
                .map_err(|e| Kind::InvalidVersion.context(e))?,
            proof: ProofConnOpenTry::new(
                proof_init,
                proof_consensus,
                proof_height,
            ),
            consensus_height,
            signer,
        })
    }
}

impl Msg for MsgConnectionOpenTry {
    type ValidationError = Error;

    fn route(&self) -> String {
        crate::keys::ROUTER_KEY.to_string()
    }

    fn get_type(&self) -> String {
        TYPE_MSG_CONNECTION_OPEN_TRY.to_string()
    }

    fn validate_basic(&self) -> Result<(), Self::ValidationError> {
        self.counterparty.validate_basic()
    }

    fn get_sign_bytes(&self) -> Vec<u8> {
        unimplemented!()
    }

    fn get_signers(&self) -> Vec<AccountId> {
        vec![self.signer]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProofConnOpenTry {
    proof_init: CommitmentProof,
    proof_consensus: CommitmentProof,
    proof_height: u64,
}

impl ProofConnOpenTry {
    pub fn new(
        proof_init: CommitmentProof,
        proof_consensus: CommitmentProof,
        proof_height: u64,
    ) -> Self {
        Self{
            proof_init,
            proof_consensus,
            proof_height
        }
    }
}

pub const TYPE_MSG_CONNECTION_OPEN_ACK: &str = "connection_open_ack";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MsgConnectionOpenAck {
    connection_id: ConnectionId,
    proof: ProofConnOpenAck,
    consensus_height: u64,
    version: String,
    signer: AccountId,
}

impl MsgConnectionOpenAck {
    pub fn new(
        connection_id: String,
        proof_try: CommitmentProof,
        proof_consensus: CommitmentProof,
        proof_height: u64,
        consensus_height: u64,
        version: String,
        signer: AccountId,
    ) -> Result<MsgConnectionOpenAck, Error> {
        if consensus_height == 0 {
            fail!(Kind::InvalidHeight, "Height cannot be zero");
        }

        Ok(Self {
            connection_id: connection_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            proof: ProofConnOpenAck::new(
                proof_try,
                proof_consensus,
                proof_height,
            ),
            consensus_height,
            version: validate_version(version)
                .map_err(|e| Kind::InvalidVersion.context(e))?,
            signer,
        })
    }
}

impl Msg for MsgConnectionOpenAck {
    type ValidationError = Error;

    fn route(&self) -> String {
        crate::keys::ROUTER_KEY.to_string()
    }

    fn get_type(&self) -> String {
        TYPE_MSG_CONNECTION_OPEN_ACK.to_string()
    }

    fn validate_basic(&self) -> Result<(), Self::ValidationError> {
        //todo: validate proof !
        Ok(())
    }

    fn get_sign_bytes(&self) -> Vec<u8> {
        unimplemented!()
    }

    fn get_signers(&self) -> Vec<AccountId> {
        vec![self.signer]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProofConnOpenAck {
    proof_try: CommitmentProof,
    proof_consensus: CommitmentProof,
    proof_height: u64,
}

impl ProofConnOpenAck {
    pub fn new(
        proof_try: CommitmentProof,
        proof_consensus: CommitmentProof,
        proof_height: u64,
    ) -> Self {
        Self{
            proof_try,
            proof_consensus,
            proof_height
        }
    }
}


pub const TYPE_MSG_CONNECTION_OPEN_CONFIRM: &str = "connection_open_confirm";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MsgConnectionOpenConfirm {
    connection_id: ConnectionId,
    proof: ProofConnOpenConfirm,
    signer: AccountId,
}

impl MsgConnectionOpenConfirm {
    pub fn new(
        connection_id: String,
        proof_ack: CommitmentProof,
        proof_height: u64,
        signer: AccountId,
    ) -> Result<MsgConnectionOpenConfirm, Error> {
        if proof_height == 0 {
            fail!(Kind::InvalidHeight, "Height cannot be zero");
        }

        Ok(Self {
            connection_id: connection_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            proof: ProofConnOpenConfirm::new(
                proof_ack,
                proof_height,
            ),
            signer,
        })
    }
}

impl Msg for MsgConnectionOpenConfirm {
    type ValidationError = Error;

    fn route(&self) -> String {
        crate::keys::ROUTER_KEY.to_string()
    }

    fn get_type(&self) -> String {
        TYPE_MSG_CONNECTION_OPEN_CONFIRM.to_string()
    }

    fn validate_basic(&self) -> Result<(), Self::ValidationError> {
        //todo: validate proof !
        Ok(())
    }

    fn get_sign_bytes(&self) -> Vec<u8> {
        unimplemented!()
    }

    fn get_signers(&self) -> Vec<AccountId> {
        vec![self.signer]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProofConnOpenConfirm {
    proof_ack: CommitmentProof,
    proof_height: u64,
}

impl ProofConnOpenConfirm {
    pub fn new(
        proof_ack: CommitmentProof,
        proof_height: u64,
    ) -> Self {
        Self{
            proof_ack,
            proof_height
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ics23_commitment::CommitmentPrefix;
    use std::str::FromStr;
    use tendermint::account::Id as AccountId;
    use super::MsgConnectionOpenInit;

    #[derive(Clone, Debug, PartialEq)]
    struct ConOpenInitParams {
        connection_id: String,
        client_id: String,
        counterparty_connection_id: String,
        counterparty_client_id: String,
        counterparty_commitment_prefix: CommitmentPrefix,
    }

    struct Test {
        name: String,
        params: ConOpenInitParams,
        want_pass: bool,
    }

    fn default_test_setting() -> (ConOpenInitParams, AccountId) {

        let id_hex = "0CDA3F47EF3C4906693B170EF650EB968C5F4B2C";
        let acc = AccountId::from_str(id_hex).unwrap();


        let default_con_params = ConOpenInitParams {
            connection_id: "srcconnection".to_string(),
            client_id: "srcclient".to_string(),
            counterparty_connection_id: "destconnection".to_string(),
            counterparty_client_id: "destclient".to_string(),
            counterparty_commitment_prefix: CommitmentPrefix {},
        };



        (default_con_params, acc)
    }

    #[test]
    fn parse_connection_open_init_msg() {

        let (default_con_params, acc) = default_test_setting();

        let tests: Vec<Test> = vec![
            Test {
                name: "Good parameters".to_string(),
                params: default_con_params.clone(),
                want_pass: true,
            },
            Test {
                name: "Bad connection id, non-alpha".to_string(),
                params: ConOpenInitParams {
                    connection_id: "con007".to_string(),
                    ..default_con_params.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad client id, name too short".to_string(),
                params: ConOpenInitParams {
                    client_id: "client".to_string(),
                    ..default_con_params.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad destination connection id, name too long".to_string(),
                params: ConOpenInitParams {
                    counterparty_connection_id: "abcdefghijklmnopqrstu".to_string(),
                    ..default_con_params.clone()
                },
                want_pass: false,
            },
        ]
            .into_iter()
            .collect();

        for test in tests {
            let p = test.params.clone();

            let msg = MsgConnectionOpenInit::new(
                p.connection_id,
                p.client_id,
                p.counterparty_connection_id,
                p.counterparty_client_id,
                p.counterparty_commitment_prefix,
                acc,
            );

            assert_eq!(
                test.want_pass, msg.is_ok(),
                "MsgConnOpenInit::new failed for test {}, \nmsg {:?}",
                test.name,
                test.params.clone()
            );
        }
    }
}
