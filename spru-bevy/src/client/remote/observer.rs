use bevy::prelude;

use crate::client;

pub fn on_connected<Client: client::ClientSSS>(
    trigger: prelude::On<prelude::Add, aeronet::io::Session>, 
    mut commands: prelude::Commands,
    q_client: prelude::Query<(
        &aeronet::io::Session,
    )>,
) -> prelude::Result {
    let client = trigger.entity;
    let Ok((session, )) = q_client.get(client) else {
        return Ok(());
    };

    commands.entity(client)
        .insert(aeronet::transport::Transport::new(
            session,
            crate::remote::SERVER_TO_CLIENT_LANES,
            crate::remote::CLIENT_TO_SERVER_LANES,
            bevy::platform::time::Instant::now(),
        )?);

    prelude::info!("{client} connected to server");

    Ok(())
}