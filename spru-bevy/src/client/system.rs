use bevy::prelude;

use crate::{client, common};

pub fn run_client<Client: crate::client::ClientSSS>(
    world: &mut prelude::World,
    q_client: &mut prelude::QueryState<(
        prelude::Entity,
        &mut super::component::Runner<Client>,
        &common::component::GameId,
        &super::component::ClientId,
    )>,
) -> prelude::Result {
    let entities: Vec<_> = q_client.iter(world).map(|tup| tup.0).collect();

    for entity in entities {
        let mut entity = world.entity_mut(entity);
        let signals = entity.get_components_mut::<&mut client::component::FromServer<Client>>()?.take();
        for signal in signals {
            match client::component::Runner::<Client>::storage_scope(&mut entity, |client, storage| {
                client.signal(storage, signal)
            })? {
                Ok(output) => {
                    let spru::client::Output {
                        outbound,
                        events,
                        ret,
                    } = output;

                    client::trigger_events(entity.id(), &mut entity, events);

                    entity.get_components_mut::<&mut client::component::ToServer<Client>>()?
                        .enqueue_outbound(outbound);

                    let _: () = ret;
                },
                Err(err) => {
                    // TODO a signal error is fatal, we should attempt to reseed from the server
                    prelude::error!("Failed to apply signal: {err}");
                    break;
                },
            }
        }
    }

    Ok(())
}

