use abscissa_core::{Command, Options, Runnable};

use ibc::ics02_client::client_state::ClientState;
use ibc::ics24_host::identifier::{ChainId, ChannelId, PortId};
use ibc_relayer::chain::counterparty::{channel_connection_client, unreceived_acknowledgements};

use crate::cli_utils::spawn_chain_runtime;
use crate::conclude::Output;
use crate::error::{Error, Kind};
use crate::prelude::*;

/// This command does the following:
/// 1. queries the chain to get its counterparty, channel and port identifiers (needed in 2)
/// 2. queries the chain for all packet commitments/ sequences for a given port and channel
/// 3. queries the counterparty chain for the unacknowledged sequences out of the list obtained in 2.
#[derive(Clone, Command, Debug, Options)]
pub struct QueryUnreceivedAcknowledgementCmd {
    #[options(
        free,
        required,
        help = "identifier of the chain to query the unreceived acknowledgments"
    )]
    chain_id: ChainId,

    #[options(free, required, help = "port identifier")]
    port_id: PortId,

    #[options(free, required, help = "channel identifier")]
    channel_id: ChannelId,
}

impl QueryUnreceivedAcknowledgementCmd {
    fn execute(&self) -> Result<Vec<u64>, Error> {
        let config = app_config();
        debug!("Options: {:?}", self);

        let chain = spawn_chain_runtime(&config, &self.chain_id)?;

        let channel_connection_client =
            channel_connection_client(chain.as_ref(), &self.port_id, &self.channel_id)
                .map_err(|e| Kind::Query.context(e))?;
        let channel = channel_connection_client.channel;
        debug!(
            "fetched from source chain {} the following channel {:?}",
            self.chain_id, channel
        );

        let counterparty_chain = {
            let counterparty_chain_id = channel_connection_client.client.client_state.chain_id();
            spawn_chain_runtime(&config, &counterparty_chain_id)?
        };

        unreceived_acknowledgements(chain.as_ref(), counterparty_chain.as_ref(), channel)
            .map_err(|e| Kind::Query.context(e).into())
    }
}

impl Runnable for QueryUnreceivedAcknowledgementCmd {
    fn run(&self) {
        match self.execute() {
            Ok(seqs) => Output::success(seqs).exit(),
            Err(e) => Output::error(format!("{}", e)).exit(),
        }
    }
}
