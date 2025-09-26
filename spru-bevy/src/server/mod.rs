pub mod command;
pub mod component;
mod plugin;
pub use plugin::Plugin;
pub mod query;
pub mod system;

pub trait ServerSSS: 
    spru::server::Bounded<
        Action: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        PlayerInit: spru::player::Init<In: Send + Sync + 'static> + Send + Sync + 'static,
        Reaction: spru::Reaction<GameOutcome: Send + Sync + 'static> + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        State: for<'l> spru::State<crate::client::BevyLookup<'l>, Repr: TryFrom<spru::state::Index>> + 'static
    > + Send + Sync + 'static
{

}

impl<Server:
    spru::server::Bounded<
        Action: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        PlayerInit: spru::player::Init<In: Send + Sync + 'static> + Send + Sync + 'static,
        Reaction: spru::Reaction<GameOutcome: Send + Sync + 'static> + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        State: for<'l> spru::State<crate::client::BevyLookup<'l>, Repr: TryFrom<spru::state::Index>> + 'static
    > + Send + Sync + 'static
> ServerSSS for Server { }