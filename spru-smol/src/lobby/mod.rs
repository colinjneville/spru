pub mod client;
mod member_status;
use futures_lite::FutureExt as _;
pub use member_status::MemberStatus;
mod ready_status;
pub use ready_status::ReadyStatus;
// use spru_message::{payload, Message};
pub mod server;

use std::{collections::{hash_map, HashMap}, ops, sync::Arc};
use serde::de::DeserializeOwned;
use crate::{lobby, router::{self, Routed}, util, Router};

pub struct Status<LobbyInfo, MemberInfo> {
    state: smol::lock::RwLock<Option<State<LobbyInfo, MemberInfo>>>,
    event: event_listener::Event,
}

impl<LobbyInfo, MemberInfo> Status<LobbyInfo, MemberInfo> {
    fn new(lobby_info: LobbyInfo) -> Self {
        let state = smol::lock::RwLock::new(Some(State::new(lobby_info)));
        let event = event_listener::Event::new();

        Self {
            state,
            event,
        }
    }

    pub async fn wait_for_update(&self) {
        self.event.listen().await
    }

    fn notify_update(&self) {
        self.event.notify(usize::MAX);
    }
}

#[derive(Debug)]
pub struct State<LobbyInfo, MemberInfo> {
    lobby_info: LobbyInfo,
    members_status: MembersStatus<MemberInfo>,
}

impl<LobbyInfo, MemberInfo> State<LobbyInfo, MemberInfo> {
    fn new(lobby_info: LobbyInfo) -> Self {
        let members_status = MembersStatus::new();
        Self {
            lobby_info,
            members_status,
        }
    }

    pub fn lobby_info(&self) -> &LobbyInfo {
        &self.lobby_info
    }

    pub fn members_status(&self) -> &MembersStatus<MemberInfo> {
        &self.members_status
    }

    fn into_output(self) -> Output<LobbyInfo, MemberInfo> {
        let Self {
            lobby_info,
            members_status,
        } = self;
        
        let members_info = members_status.state.into_iter()
            .map(|(k, v)| Routed::new(k, v.member_info))
            .collect();

        Output {
            lobby_info,
            members_info,
        }
    }
}

#[derive(Debug)]
pub struct MembersStatus<MemberInfo> {
    state: HashMap<router::Id, MemberStatus<MemberInfo>>,
}

impl<MemberInfo> MembersStatus<MemberInfo> {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    fn add_or_update(&mut self, router_id: router::Id, member_info: MemberInfo, ready_status: &ReadyStatus) -> Result<&MemberStatus<MemberInfo>, ()>{
        match self.state.entry(router_id) {
            hash_map::Entry::Occupied(oe) => {
                let m = oe.into_mut();
                m.member_info = member_info;
                Ok(m)
            }
            hash_map::Entry::Vacant(ve) => {
                let member_status = MemberStatus {
                    member_info,
                    ready_status: ready_status.try_clone()?,
                };
                Ok(ve.insert(member_status))
            }
        }
    }

    fn remove(&mut self, router_id: router::Id) -> Option<MemberStatus<MemberInfo>> {
        self.state.remove(&router_id)
    }

    pub fn get(&self, router_id: router::Id) -> Option<&MemberStatus<MemberInfo>> {
        self.state.get(&router_id)
    }

    fn get_mut(&mut self, router_id: router::Id) -> Option<&mut MemberStatus<MemberInfo>> {
        self.state.get_mut(&router_id)
    }
}

pub struct Output<LobbyInfo, MemberInfo> {
    pub lobby_info: LobbyInfo,
    pub members_info: Vec<Routed<MemberInfo>>,
}

pub struct Lobby<LobbyInfo, MemberInfo, Payload> {
    router: Router<Payload>,
    ready_status: ReadyStatus,
    all_ready_event: Arc<event_listener::Event>,
    status: Arc<Status<LobbyInfo, MemberInfo>>,
}

impl<LobbyInfo, MemberInfo, Payload> Lobby<LobbyInfo, MemberInfo, Payload> {
    pub fn new(
            router: Router<Payload>,
            lobby_info: LobbyInfo,
        ) -> Self {

        let all_ready_event = Arc::new(event_listener::Event::new());

        let ready_status = ReadyStatus::new(all_ready_event.clone());

        let status = Arc::new(Status::new(lobby_info));
        
        Self {
            router,
            ready_status,
            all_ready_event,
            status,
        }
    }

    pub fn status(&self) -> Arc<Status<LobbyInfo, MemberInfo>> {
        self.status.clone()
    }

    pub fn ready_status(&self) -> ReadyStatus {
        self.ready_status.try_clone()
            .expect("all ready cannot happen before run")
    }

    pub async fn run(self) -> Result<Output<LobbyInfo, MemberInfo>, ()> 
    where 
        MemberInfo: DeserializeOwned + Send + 'static,
        Payload: From<lobby::client::Variant<MemberInfo>>,
        lobby::client::Variant<MemberInfo>: TryFrom<Payload>,
    {
        let Self {
            router,
            mut ready_status,
            all_ready_event,
            status,
        } = self;

        // Our 'origin' ready was kept unready until `run`. At this point,
        // either the lobby setup or router should have their own `ReadyStatus`
        // to keep the lobby open as needed.
        if ready_status.ready().await {
            // TODO log a warning explaining `ReadyStatus`
        }

        let status_clone = status.clone();

        all_ready_event.listen().or(
            async move {
                loop {
                    let Routed { client_id, value: message } = match router.recv().await {
                        Ok(routed) => routed,
                        Err(_) => 
                            // TODO error handling
                            continue,
                    };
                    let Ok(message): Result<client::Variant<MemberInfo>, _> = message.try_into() else {
                        // TODO
                        panic!();
                    };

                    let mut state_lock = status.state.write().await;

                    let state = state_lock
                        .as_mut()
                        .expect("State must persist until the Lobby concludes");

                    match message {
                        client::Variant::UpdateInfo(member_info) => {
                            if state.members_status.add_or_update(client_id, member_info, &ready_status).is_ok() {
                                status.notify_update();
                            }
                        },
                        client::Variant::SetReady(ready) => {
                            match state.members_status.get_mut(client_id) {
                                Some(member_status) => {
                                    match member_status.ready_status.set_ready(ready).await {
                                        Ok(was_updated) => 
                                            if was_updated {
                                                status.notify_update();
                                            },
                                        Err(_) => {
                                            // TODO error handling
                                            // Already all-ready
                                        }
                                    }
                                },
                                None => {
                                    // TODO error handling
                                    // member_info must be send first
                                },
                            }
                        },
                    }
                }
            }
        ).await;

        let state = status_clone.state.write().await
            .take()
            .expect("State must persist until the Lobby concludes");

        Ok(state.into_output())
    }
}

#[cfg(test)]
mod test {
    use futures_lite::FutureExt;
    use tagset::tagset;

    use crate::Lobby;
    use super::*;

    #[tagset(impl tagset::proxy::serde::Serialize)]
    #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
    #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
    #[tagset(crate::lobby::client::Variant<i32>)]
    struct ClientPload;

    #[test]
    fn run_lobby() {
        let executor = smol::LocalExecutor::new();

        let router = Router::new();
        let router_tcp_listener = router.create_tcp_listener((std::net::Ipv4Addr::LOCALHOST, 0));

        let local_stream = smol::block_on(router.create_local_connection());
        let local_stream2 = smol::block_on(router.create_local_connection());

        let lobby_info = 7u32;

        

        let lobby = Lobby::<_, i32, ClientPload>::new(router, lobby_info);

        let lobby_status = lobby.status();

        let mut lobby_ready_status = lobby.ready_status();

        executor.spawn(router_tcp_listener.listen())
            .detach();

        executor.spawn(async move {
            println!("[1.1]");
            smol::Timer::after(std::time::Duration::from_millis(500)).await;
            println!("[1.2]");
            local_stream.send(client::Variant::UpdateInfo(111).into()).await.unwrap();
            println!("[1.3]");
            smol::Timer::after(std::time::Duration::from_millis(250)).await;
            println!("[1.4]");
            local_stream.send(client::Variant::SetReady(true).into()).await.unwrap();
            println!("[1.5]");
        }).detach();

        executor.spawn(async move {
            smol::Timer::after(std::time::Duration::from_millis(100)).await;
            local_stream2.send(client::Variant::UpdateInfo(222).into()).await.unwrap();
            smol::Timer::after(std::time::Duration::from_millis(750)).await;
            local_stream2.send(client::Variant::SetReady(true).into()).await.unwrap();
        }).detach();

        executor.spawn(async move {
            smol::Timer::after(std::time::Duration::from_millis(200)).await;
            lobby_ready_status.ready().await;
        }).detach();

        let members = smol::block_on(executor.run(
            async move { Some(lobby.run().await) }.or(
                async move { smol::Timer::after(std::time::Duration::from_secs(5)).await; None})))
            .expect("lobby did not complete in time limit")
            .expect("lobby failed");

        assert_eq!(members.members_info[0].value + members.members_info[1].value, 333);

    }
}