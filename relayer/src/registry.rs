//! Registry for keeping track of [`ChainHandle`]s indexed by a `ChainId`.

use std::{collections::HashMap, sync::Arc};

use anomaly::BoxError;
use tokio::runtime::Runtime as TokioRuntime;
use tracing::{trace, warn};

use ibc::ics24_host::identifier::ChainId;

use crate::{
    chain::{handle::ChainHandle, runtime::ChainRuntime, CosmosSdkChain},
    config::Config,
    supervisor::RwArc,
};

/// Registry for keeping track of [`ChainHandle`]s indexed by a `ChainId`.
///
/// The purpose of this type is to avoid spawning multiple runtimes for a single `ChainId`.
pub struct Registry {
    config: RwArc<Config>,
    handles: HashMap<ChainId, Box<dyn ChainHandle>>,
    rt: Arc<TokioRuntime>,
}

impl Registry {
    /// Construct a new [`Registry`] using the provided [`Config`]
    pub fn new(config: RwArc<Config>) -> Self {
        Self {
            config,
            handles: HashMap::new(),
            rt: Arc::new(TokioRuntime::new().unwrap()),
        }
    }

    /// Return the size of the registry, i.e., the number of distinct chain runtimes.
    pub fn size(&self) -> usize {
        self.handles.len()
    }

    /// Return an iterator overall the chain handles managed by the registry.
    pub fn chains(&self) -> impl Iterator<Item = &Box<dyn ChainHandle>> {
        self.handles.values()
    }

    /// Get the [`ChainHandle`] associated with the given [`ChainId`].
    ///
    /// If there is no handle yet, this will first spawn the runtime and then
    /// return its handle.
    pub fn get_or_spawn(&mut self, chain_id: &ChainId) -> Result<Box<dyn ChainHandle>, BoxError> {
        self.spawn(chain_id)?;

        let handle = self
            .handles
            .get(chain_id)
            .expect("runtime was just spawned");

        Ok(handle.clone())
    }

    /// Spawn a chain runtime for the chain with the given [`ChainId`],
    /// only if the registry does not contain a handle for that runtime already.
    ///
    /// Returns whether or not the runtime was actually spawned.
    pub fn spawn(&mut self, chain_id: &ChainId) -> Result<bool, BoxError> {
        if !self.handles.contains_key(chain_id) {
            let handle = spawn_chain_runtime(&self.config, chain_id, self.rt.clone())?;
            self.handles.insert(chain_id.clone(), handle);
            trace!("spawned chain runtime for chain {}", chain_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Shutdown the runtime associated with the given chain identifier.
    pub fn shutdown(&mut self, chain_id: &ChainId) {
        if let Some(handle) = self.handles.remove(chain_id) {
            if let Err(e) = handle.shutdown() {
                warn!(chain.id = %chain_id, "chain runtime might have failed to shutdown properly: {}", e);
            }
        }
    }
}

/// Spawns a chain runtime from the configuration and given a chain identifier.
/// Returns the corresponding handle if successful.
pub fn spawn_chain_runtime(
    config: &RwArc<Config>,
    chain_id: &ChainId,
    rt: Arc<TokioRuntime>,
) -> Result<Box<dyn ChainHandle>, BoxError> {
    let chain_config = config
        .read()
        .expect("poisoned lock")
        .find_chain(chain_id)
        .cloned()
        .ok_or_else(|| format!("missing chain for id ({}) in configuration file", chain_id))?;

    let handle = ChainRuntime::<CosmosSdkChain>::spawn(chain_config, rt)?;

    Ok(handle)
}
