pub mod command;
pub mod component;
mod plugin;
pub use plugin::Plugin;
pub mod query;
pub mod system;

pub trait ServerSSS: 
    spru::Server<
        State: spru::State + Send + Sync + 'static,
        Action: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        PlayerInit: spru::player::Init<In: Send + Sync + 'static> + Send + Sync + 'static,
        Reaction: spru::Reaction<GameOutcome: Send + Sync + 'static> + Send + Sync + 'static,
        Root: Send + Sync + 'static,
    > + Send + Sync + 'static
{

}

impl<Server:
    spru::Server<
        State: spru::State + Send + Sync + 'static,
        Action: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        PlayerInit: spru::player::Init<In: Send + Sync + 'static> + Send + Sync + 'static,
        Reaction: spru::Reaction<GameOutcome: Send + Sync + 'static> + Send + Sync + 'static,
        Root: Send + Sync + 'static,
    > + Send + Sync + 'static
> ServerSSS for Server { }