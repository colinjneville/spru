use derive_where::derive_where;

#[derive_where(Debug; GameComplete<Client>)]
#[derive(derive_more::From)]
#[non_exhaustive]
pub enum Event<Client: super::Client> {
    GameComplete(GameComplete<Client>),
}

#[derive_where(Debug; Client::GameOutcome)]
pub struct GameComplete<Client: super::Client> {
    pub game_outcome: Client::GameOutcome,
}

// #[allow(unused_variables)]
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

//     fn game_complete(&mut self, event: GameComplete<GameOutcome>) { }
// }