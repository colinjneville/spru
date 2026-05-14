use spru_util::{counter, fsm, pile, player_map, rotating, state_cell};
use tagset::tagset;

use crate::{data, game, player, reaction, round};

#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(impl spru::State)]
#[tagset(impl<Action, Registry> spru_script::Scriptable<Action, Registry> 
    where
        Registry: 
            spru_script::RegistryType<Self, Action, player::Root> +
            spru_script::RegistryType<Self, Action, player::Input> +
            spru_script::RegistryType<Self, Action, player::machine::Input> +
            spru_script::RegistryType<Self, Action, round::machine::Input> +
            spru_script::RegistryType<Self, Action, data::Card> +
            spru_script::RegistryType<Self, Action, crate::Play> +
            spru_script::RegistryType<Self, Action, reaction::Trigger> +
        ,
    {
        fn register<Storage>(registry: &Registry, registration: &mut Registry::Registration<'_, Storage>) 
            -> Result<(), Registry::Error>
        where
            Storage: spru::item::Storage<State = Self>,
        {
            foreach!(VAR => {
                spru_script::RegistryState::<Self, Action, VAR>::register_state::<Storage>(
                    registry, 
                    registration, 
                    Some(spru_script::scriptable_path!(VAR))
                )?;
            });
            spru_script::RegistryType::<Self, Action, player::Root>::register_type(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(player::Root))
            )?;
            spru_script::RegistryType::<Self, Action, player::Input>::register_type(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(player::Input))
            )?;
            spru_script::RegistryType::<Self, Action, player::machine::Input>::register_type(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(player::Machine))
            )?;
            spru_script::RegistryType::<Self, Action, round::machine::Input>::register_type(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(round::Machine))
            )?;
            spru_script::RegistryType::<Self, Action, data::Card>::register_type(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(data::Card))
            )?;
            // TODO give Play a proper path
            spru_script::RegistryType::<Self, Action, crate::Play>::register_type(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(Play))
            )?;
            spru_script::RegistryType::<Self, Action, reaction::Trigger>::register_type(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(reaction::Trigger))
            )?;
            Ok(())
        }
    }
)]
#[tagset(game::Root)]
#[tagset(player_map::PlayerMap<player::Root>)]
#[tagset(fsm::Fsm<player::machine::Impl>)]
#[tagset(fsm::Fsm<round::machine::Impl>)]
#[tagset(pile::Pile<data::Card>)]
#[tagset(counter::Counter<u32>)]
#[tagset(rotating::Rotating<spru::player::Id>)]
#[tagset(state_cell::StateCell<crate::Play>)]
#[tagset(derive(Debug))]
pub struct State;
