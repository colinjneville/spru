use bevy::prelude;

pub fn run_client<Client: crate::client::ClientSSS>(
    world: &mut prelude::World,
) 
    -> spru::client::signal::Result<()>
where 
{
    // We need to query Clients and also have access to World.
    // First create a list of all Client entities. Then query
    // each Entity individually, temporarily remove the Client
    // and EntityMap, run the Client with World, then restore 
    // the Client/EntityMap and enqueue Signals to the Server.

    let mut q_client = world.query::<
        (
            prelude::Entity,
            &mut super::component::Runner<Client>, 
            &mut super::component::FromServer<Client>,
            &mut super::component::ToServer<Client>,
            &super::component::ClientId,
        )
    >();

    let entities: Vec<_> = q_client.iter(world)
        .map(|tup| tup.0)
        .collect();

    for entity in entities {
        let mut to_server_signals = vec![];
        loop {
            let (_entity, mut runner, mut from_server, _to_server, client_id) = q_client.get_mut(world, entity)
                .expect("Query must remain valid");
            if let Some(signal) = from_server.dequeue() {
                let mut inner = runner.take();
                let client_id = client_id.0;
                let mut lookup = super::lookup::BevyLookup::new(world, &mut inner.entity_map, client_id);
                let result = inner.client.signal(&mut lookup, signal);

                let (_entity, mut runner, _from_server, _to_server, _client_id) = q_client.get_mut(world, entity)
                    .expect("Query must remain valid");
                runner.restore(inner);

                let out = result?;

                let spru::client::Output {
                    outbound,
                    events,
                    ret,
                } = out;

                to_server_signals.push(outbound);
            } else {
                break;
            }
        }

        if !to_server_signals.is_empty() {
            let (_entity, _runner, _from_server, mut to_server, _client_id) = q_client.get_mut(world, entity)
                .expect("Query must still be valid");
            for to_server_signal in to_server_signals.into_iter().flatten() {
                to_server.enqueue(to_server_signal);
            }
        }
    }

    Ok(())
}