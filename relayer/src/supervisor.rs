use std::{
    collections::HashMap,
    fmt,
    thread::{self, JoinHandle},
    time::Duration,
};

use anomaly::BoxError;
use crossbeam_channel::{Receiver, Select, Sender};
use tracing::{debug, error, info, trace, warn};

use ibc::events::VecIbcEvents;
use ibc::ics02_client::client_state::{ClientState, IdentifiedAnyClientState};
use ibc::ics02_client::events::UpdateClient;
use ibc::ics03_connection::connection::IdentifiedConnectionEnd;
use ibc::ics04_channel::channel::IdentifiedChannelEnd;
use ibc::ics04_channel::events::Attributes;
use ibc::ics24_host::identifier::ClientId;
use ibc::{
    events::IbcEvent,
    ics02_client::events::NewBlock,
    ics03_connection::connection::State as ConnectionState,
    ics04_channel::events::{CloseInit, SendPacket, TimeoutPacket, WriteAcknowledgement},
    ics24_host::identifier::{ChainId, ChannelId, PortId},
    Height,
};
use ibc_proto::ibc::core::channel::v1::QueryChannelsRequest;

use crate::foreign_client::{ForeignClient, ForeignClientError, MisbehaviourResults};
use crate::{
    chain::handle::ChainHandle,
    config::Config,
    event::monitor::EventBatch,
    link::{Link, LinkParameters},
    registry::Registry,
};

mod error;
pub use error::Error;

/// A command for a [`Worker`].
pub enum WorkerCmd {
    /// A batch of packet events need to be relayed
    IbcEvents { batch: EventBatch },
    /// A batch of [`NewBlock`] events need to be relayed
    NewBlock { height: Height, new_block: NewBlock },
}

/// Handle to a [`Worker`], for sending [`WorkerCmd`]s to it.
pub struct WorkerHandle {
    tx: Sender<WorkerCmd>,
    thread_handle: JoinHandle<()>,
}

impl WorkerHandle {
    /// Send a batch of packet events to the worker.
    pub fn send_events(
        &self,
        height: Height,
        events: Vec<IbcEvent>,
        chain_id: ChainId,
    ) -> Result<(), BoxError> {
        let batch = EventBatch {
            height,
            events,
            chain_id,
        };

        trace!("supervisor sends {:?}", batch);
        self.tx.send(WorkerCmd::IbcEvents { batch })?;
        Ok(())
    }

    /// Send a batch of [`NewBlock`] event to the worker.
    pub fn send_new_block(&self, height: Height, new_block: NewBlock) -> Result<(), BoxError> {
        self.tx.send(WorkerCmd::NewBlock { height, new_block })?;
        Ok(())
    }

    /// Wait for the worker thread to finish.
    pub fn join(self) -> thread::Result<()> {
        self.thread_handle.join()
    }
}

/// A pair of [`ChainHandle`]s.
#[derive(Clone)]
pub struct ChainHandlePair {
    pub a: Box<dyn ChainHandle>,
    pub b: Box<dyn ChainHandle>,
}

impl ChainHandlePair {
    /// Swap the two handles.
    pub fn swap(self) -> Self {
        Self {
            a: self.b,
            b: self.a,
        }
    }
}

fn recv_multiple<K, T>(rs: &[(K, Receiver<T>)]) -> Result<(&K, T), BoxError> {
    // Build a list of operations.
    let mut sel = Select::new();
    for (_, r) in rs {
        sel.recv(r);
    }

    // Complete the selected operation.
    let oper = sel.select();
    let index = oper.index();

    let (k, r) = &rs[index];

    let result = oper.recv(r)?;

    Ok((k, result))
}

/// The supervisor listens for events on multiple pairs of chains,
/// and dispatches the events it receives to the appropriate
/// worker, based on the [`Object`] associated with each event.
pub struct Supervisor {
    config: Config,
    registry: Registry,
    workers: HashMap<Object, WorkerHandle>,
}

impl Supervisor {
    /// Spawns a [`Supervisor`] which will listen for events on all the chains in the [`Config`].
    pub fn spawn(config: Config) -> Result<Self, BoxError> {
        let registry = Registry::new(config.clone());

        Ok(Self {
            config,
            registry,
            workers: HashMap::new(),
        })
    }

    /// Collect the events we are interested in from an [`EventBatch`],
    /// and maps each [`IbcEvent`] to their corresponding [`Object`].
    pub fn collect_events(
        &self,
        src_chain: &dyn ChainHandle,
        batch: EventBatch,
    ) -> CollectedEvents {
        let mut collected = CollectedEvents::new(batch.height, batch.chain_id);

        for event in batch.events {
            match event {
                IbcEvent::NewBlock(_) => {
                    collected.new_block = Some(event);
                }
                IbcEvent::UpdateClient(ref update) => {
                    if let Ok(object) = Object::for_update_client(update, src_chain) {
                        // Collect update client events only if the worker exists
                        if self.workers.get(&object).is_some() {
                            collected.per_object.entry(object).or_default().push(event);
                        }
                    }
                }
                IbcEvent::OpenAckChannel(ref open_ack) => {
                    // Create client worker here as channel end must be opened
                    if let Ok(object) =
                        Object::for_chan_open_events(open_ack.attributes(), src_chain)
                    {
                        collected.per_object.entry(object).or_default().push(event);
                    }
                }
                IbcEvent::OpenConfirmChannel(ref open_confirm) => {
                    // Create client worker here as channel end must be opened
                    if let Ok(object) =
                        Object::for_chan_open_events(open_confirm.attributes(), src_chain)
                    {
                        collected.per_object.entry(object).or_default().push(event);
                    }
                }
                IbcEvent::SendPacket(ref packet) => {
                    if let Ok(object) = Object::for_send_packet(packet, src_chain) {
                        collected.per_object.entry(object).or_default().push(event);
                    }
                }
                IbcEvent::TimeoutPacket(ref packet) => {
                    if let Ok(object) = Object::for_timeout_packet(packet, src_chain) {
                        collected.per_object.entry(object).or_default().push(event);
                    }
                }
                IbcEvent::WriteAcknowledgement(ref packet) => {
                    if let Ok(object) = Object::for_write_ack(packet, src_chain) {
                        collected.per_object.entry(object).or_default().push(event);
                    }
                }
                IbcEvent::CloseInitChannel(ref packet) => {
                    if let Ok(object) = Object::for_close_init_channel(packet, src_chain) {
                        collected.per_object.entry(object).or_default().push(event);
                    }
                }
                _ => (),
            }
        }

        collected
    }

    fn spawn_workers(&mut self) -> Result<(), BoxError> {
        let req = QueryChannelsRequest {
            pagination: ibc_proto::cosmos::base::query::pagination::all(),
        };

        for chain_config in &self.config.clone().chains {
            let chain = self.registry.get_or_spawn(&chain_config.id)?;
            let channels: Vec<IdentifiedChannelEnd> = chain.query_channels(req.clone())?;

            for channel in channels {
                self.spawn_workers_for_channel(chain.clone(), channel)?;
            }
        }
        Ok(())
    }

    /// Spawns all the [`Worker`] associated to a given channel on a given chain.
    fn spawn_workers_for_channel(
        &mut self,
        chain: Box<dyn ChainHandle>,
        channel: IdentifiedChannelEnd,
    ) -> Result<(), BoxError> {
        trace!(
            "Fetching connection_client for channel {:?} of chain {}",
            channel,
            chain.id()
        );

        let client_res =
            channel_connection_client(&channel.port_id, &channel.channel_id, chain.as_ref());

        let client = match client_res {
            Ok(conn_client) => conn_client.client,
            Err(Error::ConnectionNotOpen(..)) | Err(Error::ChannelNotOpen(..)) => {
                // These errors are silent.
                // Simply ignore the channel and return without spawning the workers.
                warn!(
                    "ignoring channel {} because it is not open (or its connection is not open)",
                    channel.channel_id
                );
                return Ok(());
            }
            Err(e) => {
                // Propagate errors.
                return Err(format!(
                    "unable to spawn workers for channel/chain pair '{}'/'{}' due to error: {:?}",
                    channel.channel_id,
                    chain.id(),
                    e
                )
                .into())
            }
        };

        trace!("Obtained client id {:?}", client.client_id);

        if self
            .config
            .find_chain(&client.client_state.chain_id())
            .is_none()
        {
            // Ignore channel, since it does not correspond to any chain in the config file
            return Ok(());
        }
        let counterparty_chain = self
            .registry
            .get_or_spawn(&client.client_state.chain_id())?;
        let pairs = ChainHandlePair {
            a: chain.clone(),
            b: counterparty_chain.clone(),
        };

        // create the client object and spawn worker
        let client_object = Object::Client(Client {
            dst_client_id: client.client_id.clone(),
            dst_chain_id: chain.id(),
            src_chain_id: client.client_state.chain_id(),
        });
        let worker_client = Worker::spawn(pairs.clone(), client_object.clone());
        self.workers.entry(client_object).or_insert(worker_client);

        // TODO(Adi): Only start the Uni worker if there are outstanding packets or ACKs.
        // create the path object and spawn worker
        let path_object = Object::UnidirectionalChannelPath(UnidirectionalChannelPath {
            dst_chain_id: counterparty_chain.id(),
            src_chain_id: chain.id(),
            src_channel_id: channel.channel_id.clone(),
            src_port_id: channel.port_id,
        });
        let worker_path = Worker::spawn(pairs, path_object.clone());
        self.workers.entry(path_object).or_insert(worker_path);

        Ok(())
    }

    /// Run the supervisor event loop.
    pub fn run(mut self) -> Result<(), BoxError> {
        let mut subscriptions = Vec::with_capacity(self.config.chains.len());

        for chain_config in &self.config.chains {
            let chain = self.registry.get_or_spawn(&chain_config.id)?;
            let subscription = chain.subscribe()?;
            subscriptions.push((chain, subscription));
        }

        self.spawn_workers()?;

        loop {
            match recv_multiple(&subscriptions) {
                Ok((chain, batch)) => {
                    self.process_batch(chain.clone(), batch.unwrap_or_clone())?;
                }
                Err(e) => {
                    dbg!(e);
                }
            }
        }
    }

    /// Process a batch of events received from a chain.
    fn process_batch(
        &mut self,
        src_chain: Box<dyn ChainHandle>,
        batch: EventBatch,
    ) -> Result<(), BoxError> {
        // TODO(ADI): Replace assert with simple equality check and return error upon mismatch.
        assert_eq!(src_chain.id(), batch.chain_id);

        let height = batch.height;
        let chain_id = batch.chain_id.clone();

        let mut collected = self.collect_events(src_chain.clone().as_ref(), batch);

        for (object, events) in collected.per_object.drain() {
            if events.is_empty() {
                continue;
            }

            debug!(
                "chain {} sent {} for object {:?}",
                chain_id,
                VecIbcEvents(events.clone()),
                object,
            );

            let src = self.registry.get_or_spawn(object.src_chain_id())?;
            let dst = self.registry.get_or_spawn(object.dst_chain_id())?;

            let worker = self.worker_for_object(object, src, dst);
            worker.send_events(height, events, chain_id.clone())?
        }

        if let Some(IbcEvent::NewBlock(new_block)) = collected.new_block {
            for (object, worker) in self.workers.iter() {
                match object {
                    // If there is a NewBlock event, forward it to certain workers.
                    Object::UnidirectionalChannelPath(p) => {
                        if p.src_chain_id == src_chain.id() {
                            worker.send_new_block(height, new_block)?;
                        }
                    }
                    Object::Client(_) => {}
                }
            }
        }

        Ok(())
    }

    /// Get a handle to the worker in charge of handling events associated
    /// with the given [`Object`].
    ///
    /// This function will spawn a new [`Worker`] if one does not exists already.
    // TODO(Adi): This had return type `Option`: was it necessary?
    fn worker_for_object(
        &mut self,
        object: Object,
        src: Box<dyn ChainHandle>,
        dst: Box<dyn ChainHandle>,
    ) -> &WorkerHandle {
        if self.workers.contains_key(&object) {
            &self.workers[&object]
        } else {
            let worker = Worker::spawn(ChainHandlePair { a: src, b: dst }, object.clone());
            let worker = self.workers.entry(object).or_insert(worker);
            worker
        }
    }
}

/// The direction in which a [`Worker`] should relay events.
#[derive(Copy, Clone, Debug)]
pub enum Direction {
    /// From chain A to chain B.
    AtoB,
    /// From chain B to chain A.
    BtoA,
}

/// A worker processes batches of events associated with a given [`Object`].
pub struct Worker {
    chains: ChainHandlePair,
    rx: Receiver<WorkerCmd>,
}

impl fmt::Display for Worker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{} <-> {}]", self.chains.a.id(), self.chains.b.id(),)
    }
}

impl Worker {
    /// Spawn a worker which relay events pertaining to an [`Object`] between two `chains`.
    pub fn spawn(chains: ChainHandlePair, object: Object) -> WorkerHandle {
        let (tx, rx) = crossbeam_channel::unbounded();

        debug!(
            "[{}] Spawned worker with chains a:{} and b:{} for object {:#?} ",
            object.short_name(),
            chains.a.id(),
            chains.b.id(),
            object,
        );

        let worker = Self { chains, rx };
        let thread_handle = std::thread::spawn(move || worker.run(object));

        WorkerHandle { tx, thread_handle }
    }

    /// Run the worker event loop.
    fn run(self, object: Object) {
        let result = match object.clone() {
            Object::UnidirectionalChannelPath(path) => self.run_uni_chan_path(path),
            Object::Client(client) => self.run_client(client),
        };

        if let Err(e) = result {
            error!("[{}] worker error: {}", object.short_name(), e);
        }
        info!("[{}] worker exits", object.short_name());
    }

    fn run_client_misbehaviour(
        &self,
        client: &ForeignClient,
        update: Option<UpdateClient>,
    ) -> bool {
        let mut skip_misbehaviour = false;
        let res = client.detect_misbehaviour_and_submit_evidence(update);
        match res {
            MisbehaviourResults::ValidClient => {}
            MisbehaviourResults::VerificationError => {
                // can retry in next call
            }
            MisbehaviourResults::EvidenceSubmitted(_events) => {
                // if evidence was submitted successfully then exit
                skip_misbehaviour = true;
            }
            MisbehaviourResults::CannotExecute => {
                // skip misbehaviour checking if chain does not have support for it (i.e. client
                // update event does not include the header)
                skip_misbehaviour = true;
            }
        };
        skip_misbehaviour
    }

    /// Run the event loop for events associated with a [`Client`].
    fn run_client(self, client: Client) -> Result<(), BoxError> {
        let mut client = ForeignClient::restore(
            &client.dst_client_id,
            self.chains.a.clone(),
            self.chains.b.clone(),
        );

        info!(
            "[{}] running client worker initial misbehaviour detection for {}",
            self, client
        );
        // initial check for evidence of misbehaviour for all updates
        let skip_misbehaviour = self.run_client_misbehaviour(&client, None);

        info!(
            "[{}] running client worker loop (misbehaviour and refresh) for {}",
            self, client
        );
        loop {
            thread::sleep(Duration::from_millis(600));
            // Run client refresh, exit only if expired or frozen
            if let Err(ForeignClientError::ExpiredOrFrozen(client_id, chain_id)) = client.refresh()
            {
                return Err(Box::new(ForeignClientError::ExpiredOrFrozen(
                    client_id, chain_id,
                )));
            }

            if skip_misbehaviour {
                continue;
            }
            if let Ok(WorkerCmd::IbcEvents { batch }) = self.rx.try_recv() {
                trace!("[{}] client worker receives batch {:?}", client, batch);

                for event in batch.events {
                    if let IbcEvent::UpdateClient(update) = event {
                        debug!("[{}] client updated", client);
                        // Run misbehaviour. If evidence submitted the loop will exit in next
                        // iteration with frozen client
                        self.run_client_misbehaviour(&client, Some(update));
                    }
                }
            }
        }
    }

    /// Run the event loop for events associated with a [`UnidirectionalChannelPath`].
    fn run_uni_chan_path(self, path: UnidirectionalChannelPath) -> Result<(), BoxError> {
        let mut link = Link::new_from_opts(
            self.chains.a.clone(),
            self.chains.b.clone(),
            LinkParameters {
                src_port_id: path.src_port_id,
                src_channel_id: path.src_channel_id,
            },
        )?;

        if link.is_closed()? {
            warn!("channel is closed, exiting");
            return Ok(());
        }

        loop {
            if let Ok(cmd) = self.rx.try_recv() {
                match cmd {
                    WorkerCmd::IbcEvents { batch } => {
                        link.a_to_b.update_schedule(batch)?;
                        // Refresh the scheduled batches and execute any outstanding ones.
                    }
                    WorkerCmd::NewBlock {
                        height,
                        new_block: _,
                    } => link.a_to_b.clear_packets(height)?,
                }
            }

            // Refresh the scheduled batches and execute any outstanding ones.
            link.a_to_b.refresh_schedule()?;
            link.a_to_b.execute_schedule()?;

            thread::sleep(Duration::from_millis(100))
        }
    }
}

/// Client
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Client {
    /// Destination chain identifier.
    pub dst_chain_id: ChainId,

    /// Source channel identifier.
    pub dst_client_id: ClientId,

    /// Source chain identifier.
    pub src_chain_id: ChainId,
}

impl Client {
    pub fn short_name(&self) -> String {
        format!(
            "{} -> {}:{}",
            self.src_chain_id, self.dst_chain_id, self.dst_client_id
        )
    }
}

/// A unidirectional path from a source chain, channel and port.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnidirectionalChannelPath {
    /// Destination chain identifier.
    pub dst_chain_id: ChainId,

    /// Source chain identifier.
    pub src_chain_id: ChainId,

    /// Source channel identifier.
    pub src_channel_id: ChannelId,

    /// Source port identifier.
    pub src_port_id: PortId,
}

impl UnidirectionalChannelPath {
    pub fn short_name(&self) -> String {
        format!(
            "{}/{}:{} -> {}",
            self.src_channel_id, self.src_port_id, self.src_chain_id, self.dst_chain_id,
        )
    }
}

/// An object determines the amount of parallelism that can
/// be exercised when processing [`IbcEvent`] between
/// two chains. For each [`Object`], a corresponding
/// [`Worker`] is spawned and all [`IbcEvent`]s mapped
/// to an [`Object`] are sent to the associated [`Worker`]
/// for processing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Object {
    /// See [`Client`].
    Client(Client),
    /// See [`UnidirectionalChannelPath`].
    UnidirectionalChannelPath(UnidirectionalChannelPath),
}

impl From<Client> for Object {
    fn from(c: Client) -> Self {
        Self::Client(c)
    }
}

impl From<UnidirectionalChannelPath> for Object {
    fn from(p: UnidirectionalChannelPath) -> Self {
        Self::UnidirectionalChannelPath(p)
    }
}

impl Object {
    pub fn src_chain_id(&self) -> &ChainId {
        match self {
            Self::Client(ref client) => &client.src_chain_id,
            Self::UnidirectionalChannelPath(ref path) => &path.src_chain_id,
        }
    }

    pub fn dst_chain_id(&self) -> &ChainId {
        match self {
            Self::Client(ref client) => &client.dst_chain_id,
            Self::UnidirectionalChannelPath(ref path) => &path.dst_chain_id,
        }
    }

    pub fn short_name(&self) -> String {
        match self {
            Self::Client(ref client) => client.short_name(),
            Self::UnidirectionalChannelPath(ref path) => path.short_name(),
        }
    }

    /// Build the object associated with the given [`UpdateClient`] event.
    pub fn for_update_client(
        e: &UpdateClient,
        dst_chain: &dyn ChainHandle,
    ) -> Result<Self, BoxError> {
        let client_state = dst_chain.query_client_state(e.client_id(), Height::zero())?;
        if client_state.refresh_period().is_none() {
            return Err(format!(
                "client '{}' on chain {} does not require refresh",
                e.client_id(),
                dst_chain.id()
            )
            .into());
        }

        let src_chain_id = client_state.chain_id();

        Ok(Client {
            dst_client_id: e.client_id().clone(),
            dst_chain_id: dst_chain.id(),
            src_chain_id,
        }
        .into())
    }

    /// Build the client object associated with the given channel event attributes.
    pub fn for_chan_open_events(
        e: &Attributes,
        dst_chain: &dyn ChainHandle,
    ) -> Result<Self, BoxError> {
        let channel_id = e
            .channel_id()
            .as_ref()
            .ok_or_else(|| format!("channel_id missing in OpenAck event '{:?}'", e))?;

        let client = channel_connection_client(e.port_id(), channel_id, dst_chain)?.client;
        if client.client_state.refresh_period().is_none() {
            return Err(format!(
                "client '{}' on chain {} does not require refresh",
                client.client_id,
                dst_chain.id()
            )
            .into());
        }

        Ok(Client {
            dst_client_id: client.client_id.clone(),
            dst_chain_id: dst_chain.id(),
            src_chain_id: client.client_state.chain_id(),
        }
        .into())
    }

    /// Build the object associated with the given [`SendPacket`] event.
    pub fn for_send_packet(e: &SendPacket, src_chain: &dyn ChainHandle) -> Result<Self, BoxError> {
        let dst_chain_id =
            get_counterparty_chain(src_chain, &e.packet.source_channel, &e.packet.source_port)?;

        Ok(UnidirectionalChannelPath {
            dst_chain_id,
            src_chain_id: src_chain.id(),
            src_channel_id: e.packet.source_channel.clone(),
            src_port_id: e.packet.source_port.clone(),
        }
        .into())
    }

    /// Build the object associated with the given [`WriteAcknowledgement`] event.
    pub fn for_write_ack(
        e: &WriteAcknowledgement,
        src_chain: &dyn ChainHandle,
    ) -> Result<Self, BoxError> {
        let dst_chain_id = get_counterparty_chain(
            src_chain,
            &e.packet.destination_channel,
            &e.packet.destination_port,
        )?;

        Ok(UnidirectionalChannelPath {
            dst_chain_id,
            src_chain_id: src_chain.id(),
            src_channel_id: e.packet.destination_channel.clone(),
            src_port_id: e.packet.destination_port.clone(),
        }
        .into())
    }

    /// Build the object associated with the given [`TimeoutPacket`] event.
    pub fn for_timeout_packet(
        e: &TimeoutPacket,
        src_chain: &dyn ChainHandle,
    ) -> Result<Self, BoxError> {
        let dst_chain_id =
            get_counterparty_chain(src_chain, &e.packet.source_channel, &e.packet.source_port)?;

        Ok(UnidirectionalChannelPath {
            dst_chain_id,
            src_chain_id: src_chain.id(),
            src_channel_id: e.src_channel_id().clone(),
            src_port_id: e.src_port_id().clone(),
        }
        .into())
    }

    /// Build the object associated with the given [`CloseInit`] event.
    pub fn for_close_init_channel(
        e: &CloseInit,
        src_chain: &dyn ChainHandle,
    ) -> Result<Self, BoxError> {
        let dst_chain_id = get_counterparty_chain(src_chain, e.channel_id(), &e.port_id())?;

        Ok(UnidirectionalChannelPath {
            dst_chain_id,
            src_chain_id: src_chain.id(),
            src_channel_id: e.channel_id().clone(),
            src_port_id: e.port_id().clone(),
        }
        .into())
    }
}

/// Describes the result of [`collect_events`].
#[derive(Clone, Debug)]
pub struct CollectedEvents {
    /// The height at which these events were emitted from the chain.
    pub height: Height,
    /// The chain from which the events were emitted.
    pub chain_id: ChainId,
    /// [`NewBlock`] event collected from the [`EventBatch`].
    pub new_block: Option<IbcEvent>,
    /// Mapping between [`Object`]s and their associated [`IbcEvent`]s.
    pub per_object: HashMap<Object, Vec<IbcEvent>>,
}

impl CollectedEvents {
    pub fn new(height: Height, chain_id: ChainId) -> Self {
        Self {
            height,
            chain_id,
            new_block: Default::default(),
            per_object: Default::default(),
        }
    }

    /// Whether the collected events include a [`NewBlock`] event.
    pub fn has_new_block(&self) -> bool {
        self.new_block.is_some()
    }
}

pub struct ChannelConnectionClient {
    pub channel: IdentifiedChannelEnd,
    pub connection: IdentifiedConnectionEnd,
    pub client: IdentifiedAnyClientState,
}

impl ChannelConnectionClient {
    pub fn new(
        channel: IdentifiedChannelEnd,
        connection: IdentifiedConnectionEnd,
        client: IdentifiedAnyClientState,
    ) -> Self {
        ChannelConnectionClient {
            channel,
            connection,
            client,
        }
    }
}

fn channel_connection_client(
    port_id: &PortId,
    channel_id: &ChannelId,
    chain: &dyn ChainHandle,
) -> Result<ChannelConnectionClient, Error> {
    trace!(
        chain_id = %chain.id(),
        port_id = %port_id,
        channel_id = %channel_id,
        "getting counterparty chain"
    );
    let channel_end = chain
        .query_channel(port_id, channel_id, Height::zero())
        .map_err(|e| Error::QueryFailed(format!("{}", e)))?;
    if !channel_end.is_open() {
        return Err(Error::ChannelNotOpen(channel_id.clone(), chain.id()));
    }

    let channel =
        IdentifiedChannelEnd::new(port_id.clone(), channel_id.clone(), channel_end.clone());
    let connection_id = channel_end
        .connection_hops()
        .first()
        .ok_or_else(|| Error::MissingConnectionHops(channel_id.clone(), chain.id()))?;

    let connection_end = chain
        .query_connection(&connection_id, Height::zero())
        .map_err(|e| Error::QueryFailed(format!("{}", e)))?;
    if !connection_end.state_matches(&ConnectionState::Open) {
        return Err(Error::ConnectionNotOpen(
            connection_id.clone(),
            channel_id.clone(),
            chain.id(),
        ));
    }
    let connection = IdentifiedConnectionEnd::new(connection_id.clone(), connection_end.clone());

    let client_id = connection_end.client_id();
    let client_state = chain
        .query_client_state(client_id, Height::zero())
        .map_err(|e| Error::QueryFailed(format!("{}", e)))?;
    let client = IdentifiedAnyClientState::new(client_id.clone(), client_state.clone());
    trace!(
        chain_id=%chain.id(), port_id=%port_id, channel_id=%channel_id,
        "counterparty chain: {}", client_state.chain_id()
    );

    Ok(ChannelConnectionClient::new(channel, connection, client))
}

// TODO: Memoize this result
fn get_counterparty_chain(
    src_chain: &dyn ChainHandle,
    src_channel_id: &ChannelId,
    src_port_id: &PortId,
) -> Result<ChainId, BoxError> {
    let client_state = channel_connection_client(src_port_id, src_channel_id, src_chain)?
        .client
        .client_state;
    Ok(client_state.chain_id())
}
