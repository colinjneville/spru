pub mod adapt;
pub mod component;
pub mod system;

use bevy::prelude;

pub trait CommonSSS: 
    spru::Common<
        State: Send + Sync + 'static,
        Action: spru::Action<State = Self::State> + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        GameOutcome: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
    > + Send + Sync + 'static
{

}

impl<Common:
    spru::Common<
        State: Send + Sync + 'static,
        Action: spru::Action<State = Self::State> + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        GameOutcome: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
    > + Send + Sync + 'static
> CommonSSS for Common { }

pub(crate) trait TriggerEvent {
    fn trigger<'a, E: prelude::Event<Trigger<'a>: Default>>(&mut self, event: E);
}

impl TriggerEvent for prelude::World {
    fn trigger<'a, E: prelude::Event<Trigger<'a>: Default>>(&mut self, event: E) {
        self.trigger(event);
    }
}

impl TriggerEvent for prelude::Commands<'_, '_> {
    fn trigger<'a, E: prelude::Event<Trigger<'a>: Default>>(&mut self, event: E) {
        self.trigger(event);
    }
}

impl TriggerEvent for bevy::ecs::world::CommandQueue {
    fn trigger<'a, E: prelude::Event<Trigger<'a>: Default>>(&mut self, event: E) {
        self.push(bevy::ecs::system::command::trigger(event));
    }
}
