use crate::attribute;
use crate::events::{IBCEvent, TryObject};
use crate::ics02_client::client_type::ClientType;
use crate::ics24_host::identifier::ClientId;

use serde_derive::{Deserialize, Serialize};
use std::convert::TryFrom;
use tendermint::block;
use tendermint::rpc::event_listener::ResultEvent;

// TODO - find a better place for NewBlock
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NewBlock {
    pub height: block::Height,
}

impl NewBlock {
    pub fn new(h: block::Height) -> NewBlock {
        NewBlock { height: h }
    }
}

impl TryFrom<&ResultEvent> for NewBlock {
    type Error = Box<dyn std::error::Error>;
    fn try_from(result: &ResultEvent) -> Result<Self, Self::Error> {
        Ok(NewBlock {
            height: crate::events::extract_block_height(result)?,
        })
    }
}

impl From<NewBlock> for IBCEvent {
    fn from(v: NewBlock) -> Self {
        IBCEvent::NewBlock(v)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateClient {
    pub height: block::Height,
    pub client_id: ClientId,
    pub client_type: ClientType,
}

impl TryFrom<TryObject> for CreateClient {
    type Error = Box<dyn std::error::Error>;
    fn try_from(obj: TryObject) -> Result<Self, Self::Error> {
        Ok(CreateClient {
            height: obj.height,
            client_id: attribute!(obj, "create_client.client_id"),
            client_type: attribute!(obj, "create_client.client_type"),
        })
    }
}

impl From<CreateClient> for IBCEvent {
    fn from(v: CreateClient) -> Self {
        IBCEvent::CreateClient(v)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateClient {
    pub height: block::Height,
    pub client_id: ClientId,
    pub client_type: ClientType,
}

impl TryFrom<TryObject> for UpdateClient {
    type Error = Box<dyn std::error::Error>;
    fn try_from(obj: TryObject) -> Result<Self, Self::Error> {
        Ok(UpdateClient {
            height: obj.height,
            client_id: attribute!(obj, "update_client.client_id"),
            client_type: attribute!(obj, "update_client.client_type"),
        })
    }
}

impl From<UpdateClient> for IBCEvent {
    fn from(v: UpdateClient) -> Self {
        IBCEvent::UpdateClient(v)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientMisbehavior {
    pub height: block::Height,
    pub client_id: ClientId,
    pub client_type: ClientType,
}

impl TryFrom<TryObject> for ClientMisbehavior {
    type Error = Box<dyn std::error::Error>;
    fn try_from(obj: TryObject) -> Result<Self, Self::Error> {
        Ok(ClientMisbehavior {
            height: obj.height,
            client_id: attribute!(obj, "client_misbehaviour.client_id"),
            client_type: attribute!(obj, "client_misbehaviour.client_type"),
        })
    }
}

impl From<ClientMisbehavior> for IBCEvent {
    fn from(v: ClientMisbehavior) -> Self {
        IBCEvent::ClientMisbehavior(v)
    }
}
