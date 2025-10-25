use derive_where::derive_where;

#[derive_where(Debug; GameComplete<Server>)]
#[derive(derive_more::From)]
#[non_exhaustive]
pub enum Event<Server: super::Server> {
    GameComplete(GameComplete<Server>),
}

#[derive_where(Debug; <Server::Reaction as crate::Reaction>::GameOutcome)]
pub struct GameComplete<Server: super::Server> {
    pub game_outcome: <Server::Reaction as crate::Reaction>::GameOutcome,
}

// pub trait Reader<GameOutcome> {
//     fn read_all(&mut self, events: Vec<Event<GameOutcome>>) {
//         for event in events {
//             self.read(event);
//         }
//     }

//     fn read(&mut self, event: Event<GameOutcome>) {
//         match event {
//             Event::GameComplete(event) => self.game_complete(event),
//         }
//     }

//     fn game_complete(&mut self, event: GameComplete<GameOutcome>) {
//         let _ = event;
//     }
// }
