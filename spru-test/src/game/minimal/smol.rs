#[spru_message::payload_variant(0 => spru_smol::lobby::client::Variant::<MemberInfo>)]
#[spru_message::payload_variant(1 => spru::directive::Client::<PlayerData, ActionCatalog, GameOutcome>)]
#[spru_message::payload_variant(2 => spru_smol::lobby::server::Variant::<MemberInfo>)]
#[spru_message::payload_variant(3 => spru::directive::Server::<Interaction>)]
pub struct Payload;

#[cfg(test)]
mod test {
    use spru_smol::{lobby, Routed, Router};

    #[test]
    fn run_game() {
        use futures_lite::FutureExt as _;

        let executor = smol::LocalExecutor::new();

        let mut lookup = spru_util::lookup::Standalone::new();
        let router = Router::<minimal::Payload>::new();

        let lookup_mut = &mut lookup;

        let mut game = smol::block_on(async move {
            let mut connections = vec![];
            let mut members_info = vec![];
            for i in 0..4 {
                let connection = router.create_local_connection().await;
                let client_id = connection.router_id();
                members_info.push(Routed {
                    client_id,
                    value: minimal::PLAYER_COLORS[i],
                });
                connections.push(connection);
            }
            
            let lobby_info = minimal::LobbyInfo { };
            let lobby_output = lobby::Output {
                lobby_info,
                members_info,
            };
            let game_init = minimal::GameInit;
            let player_init = minimal::PlayerInit;
            let reaction = minimal::Reaction;

            let game = spru_smol::Server::new(lookup_mut, game_init, player_init, reaction, lobby_output, router.clone())
                .unwrap();

            game
        });

        executor.spawn(async move {

        });

        smol::block_on(executor.run(
            async move { Some(game.run::<_, minimal::Interaction>(&mut lookup).await) }.or(
                async move { smol::Timer::after(std::time::Duration::from_secs(5)).await; None})))
            .expect("game did not complete in time limit")
            .expect("game failed");
    }
}
