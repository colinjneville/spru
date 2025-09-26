pub mod command;
pub mod component;
mod lookup;
pub(crate) use lookup::BevyLookup;
mod plugin;
pub use plugin::Plugin;
pub mod system;

pub trait ClientSSS: 
    spru::client::Bounded<
        Action: for<'l> spru::Action<BevyLookup<'l>> + Send + Sync + 'static,
        GameOutcome: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        Root: Send + Sync + 'static,
    > + Send + Sync + 'static
{

}

impl<Client:
    spru::client::Bounded<
        Action: for<'l> spru::Action<BevyLookup<'l>> + Send + Sync + 'static,
        GameOutcome: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        Root: Send + Sync + 'static,
    > + Send + Sync + 'static
> ClientSSS for Client { }