#[derive(Debug)]
#[derive(derive_more::From)]
pub enum Event<GameOutcome> {
    GameComplete(GameComplete<GameOutcome>),
}

#[derive(Debug)]
pub struct GameComplete<GameOutcome> {
    pub game_outcome: GameOutcome,
}

#[allow(unused_variables)]
pub trait Reader<GameOutcome> {
    fn read_all(&mut self, events: Vec<Event<GameOutcome>>) {
        for event in events {
            self.read(event);
        }
    }

    fn read(&mut self, event: Event<GameOutcome>) {
        match event {
            Event::GameComplete(event) => self.game_complete(event),
        }
    }

    fn game_complete(&mut self, event: GameComplete<GameOutcome>) { }
}