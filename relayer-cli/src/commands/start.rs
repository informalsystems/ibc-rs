use abscissa_core::{Command, Options, Runnable};

use ibc_relayer::config::Config;
use ibc_relayer::supervisor::Supervisor;

use crate::conclude::Output;
use crate::prelude::*;

#[derive(Clone, Command, Debug, Options)]
pub struct StartCmd {}

impl Runnable for StartCmd {
    fn run(&self) {
        let config = app_config();

        let supervisor = spawn_supervisor(config.clone());
        match supervisor.run() {
            Ok(()) => Output::success_msg("done").exit(),
            Err(e) => Output::error(e).exit(),
        }
    }
}

#[cfg(feature = "telemetry")]
fn spawn_supervisor(config: Config) -> Supervisor {
    let telemetry = ibc_telemetry::spawn(
        config.global.telemetry_port,
        config.global.telemetry_enabled,
    );

    Supervisor::spawn_with_telemetry(config, telemetry)
}

#[cfg(not(feature = "telemetry"))]
fn spawn_supervisor(config: Config) -> Supervisor {
    Supervisor::spawn(config)
}
