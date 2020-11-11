use crate::ics04_channel::error::{Error, Kind};
use crate::ics23_commitment::commitment::CommitmentProof;
use crate::ics24_host::identifier::{ChannelId, PortId};
use crate::address::{account_to_string, string_to_account};
use crate::{proofs::Proofs, tx_msg::Msg, Height};

use ibc_proto::ibc::core::channel::v1::MsgChannelCloseConfirm as RawMsgChannelCloseConfirm;
use tendermint::account::Id as AccountId;
use tendermint_proto::DomainType;

use std::convert::TryFrom;

/// Message type for the `MsgChannelCloseConfirm` message.
const TYPE_MSG_CHANNEL_CLOSE_CONFIRM: &str = "channel_close_confirm";

///
/// Message definition for the second step in the channel close handshake (the `ChanCloseConfirm`
/// datagram).
///
#[derive(Clone, Debug, PartialEq)]
pub struct MsgChannelCloseConfirm {
    port_id: PortId,
    channel_id: ChannelId,
    proofs: Proofs,
    signer: AccountId,
}

impl MsgChannelCloseConfirm {
    pub fn new(
        port_id: String,
        channel_id: String,
        proof_init: CommitmentProof,
        proofs_height: Height,
        signer: AccountId,
    ) -> Result<MsgChannelCloseConfirm, Error> {
        Ok(Self {
            port_id: port_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            channel_id: channel_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            proofs: Proofs::new(proof_init, None, None, proofs_height)
                .map_err(|e| Kind::InvalidProof.context(e))?,
            signer,
        })
    }
}

impl Msg for MsgChannelCloseConfirm {
    type ValidationError = Error;

    fn route(&self) -> String {
        crate::keys::ROUTER_KEY.to_string()
    }

    fn get_type(&self) -> String {
        TYPE_MSG_CHANNEL_CLOSE_CONFIRM.to_string()
    }

    fn validate_basic(&self) -> Result<(), Self::ValidationError> {
        // Nothing to validate
        // All the validation is performed on creation
        Ok(())
    }

    fn get_signers(&self) -> Vec<AccountId> {
        vec![self.signer]
    }
}

impl DomainType<RawMsgChannelCloseConfirm> for MsgChannelCloseConfirm {}

#[allow(unreachable_code, unused_variables)]
impl TryFrom<RawMsgChannelCloseConfirm> for MsgChannelCloseConfirm {
    type Error = anomaly::Error<Kind>;

    fn try_from(raw_msg: RawMsgChannelCloseConfirm) -> Result<Self, Self::Error> {
        let signer =
            string_to_account(raw_msg.signer).map_err(|e| Kind::InvalidSigner.context(e))?;

        Ok(MsgChannelCloseConfirm {
            port_id: raw_msg
                .port_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            channel_id: raw_msg
                .channel_id
                .parse()
                .map_err(|e| Kind::IdentifierError.context(e))?,
            proofs: todo!(),
            signer,
        })
    }
}

impl From<MsgChannelCloseConfirm> for RawMsgChannelCloseConfirm {
    fn from(domain_msg: MsgChannelCloseConfirm) -> Self {
        RawMsgChannelCloseConfirm {
            port_id: domain_msg.port_id.to_string(),
            channel_id: domain_msg.channel_id.to_string(),
            proof_init: vec![],
            proof_height: None,
            signer: account_to_string(domain_msg.signer).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ics04_channel::msgs::chan_close_confirm::MsgChannelCloseConfirm;
    use crate::ics23_commitment::commitment::CommitmentProof;
    use crate::test_utils::get_dummy_proof;
    use crate::Height;
    use std::str::FromStr;
    use tendermint::account::Id as AccountId;

    #[test]
    fn parse_channel_close_confirm_msg() {
        let id_hex = "0CDA3F47EF3C4906693B170EF650EB968C5F4B2C";
        let acc = AccountId::from_str(id_hex).unwrap();

        #[derive(Clone, Debug, PartialEq)]
        struct CloseConfirmParams {
            port_id: String,
            channel_id: String,
            proof_init: CommitmentProof,
            proof_height: Height,
        }

        let default_params = CloseConfirmParams {
            port_id: "port".to_string(),
            channel_id: "testchannel".to_string(),
            proof_init: get_dummy_proof().into(),
            proof_height: Height {
                version_number: 0,
                version_height: 10,
            },
        };

        struct Test {
            name: String,
            params: CloseConfirmParams,
            want_pass: bool,
        }

        let tests: Vec<Test> = vec![
            Test {
                name: "Good parameters".to_string(),
                params: default_params.clone(),
                want_pass: true,
            },
            Test {
                name: "Correct port".to_string(),
                params: CloseConfirmParams {
                    port_id: "p34".to_string(),
                    ..default_params.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Bad port, name too short".to_string(),
                params: CloseConfirmParams {
                    port_id: "p".to_string(),
                    ..default_params.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad port, name too long".to_string(),
                params: CloseConfirmParams {
                    port_id:
                        "abcdefghijklmnsdfasdfasdfasdfasdgafgadsfasdfasdfasdasfdasdfsadfopqrstu"
                            .to_string(),
                    ..default_params.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Correct channel identifier".to_string(),
                params: CloseConfirmParams {
                    channel_id: "channelid34".to_string(),
                    ..default_params.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Bad channel, name too short".to_string(),
                params: CloseConfirmParams {
                    channel_id: "chshort".to_string(),
                    ..default_params.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad channel, name too long".to_string(),
                params: CloseConfirmParams {
                    channel_id:
                        "abcdefghiasdfadsfasdfgdfsadfasdasdfasdasdfasddsfasdfasdjklmnopqrstu"
                            .to_string(),
                    ..default_params.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad proof height, height = 0".to_string(),
                params: CloseConfirmParams {
                    proof_height: Height {
                        version_number: 0,
                        version_height: 0,
                    },
                    ..default_params
                },
                want_pass: false,
            },
        ]
        .into_iter()
        .collect();

        for test in tests {
            let p = test.params.clone();

            let msg = MsgChannelCloseConfirm::new(
                p.port_id,
                p.channel_id,
                p.proof_init,
                p.proof_height,
                acc,
            );

            assert_eq!(
                test.want_pass,
                msg.is_ok(),
                "MsgChanCloseConfirm::new failed for test {}, \nmsg {:?} with error {:?}",
                test.name,
                test.params.clone(),
                msg.err(),
            );
        }
    }
}
