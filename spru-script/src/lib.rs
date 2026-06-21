//! Language-agnostic support for scripting in spru. 
//! Game Items should use the included derive macros or implement the `Scriptable*` traits manually, 
//! and specific scripting implementations should implement the `Registry*` and `Registration*` traits.
mod game_init;
pub use game_init::GameInit;
mod interaction;
pub use interaction::Interaction;
pub mod marker;
mod player_init;
pub use player_init::PlayerInit;
mod reaction;
pub use reaction::Reaction;
pub mod wrap;

// cfg_select! {
//     feature = "rhai" => {
//         pub use spru_script_rhai::*;
//     }
//     _ => {
//         pub use spru_script_impl::*;
//     }
// }

pub use spru_script_base::{LanguageActual, StatelessLanguage, LanguageStatelessEval, Language, LanguageExec, LanguageEval, StatelessLexicon, Lexicon};

/// A pre-parsed path-wise representation of a type, including type arguments. Create using [macro@scriptable_path].
pub use spru_script_base::ScriptablePath;

/// Implements [ScriptableType] for the type. Apply to the struct definition and/or impl blocks.
/// Apply `#[get]` and `#[set]` attributes to fields, and `#[get]`, `#[set]`, `#[method]`, and `#[create]`
/// attributes to functions in an impl block.
/// TODO further documentation
pub use spru_script_macro::script;

/// Parses a rust type path during macro expansion. Used for implementing [Scriptable::register].
/// Not normally needed for manual use.
pub use spru_script_macro::scriptable_path;

#[doc(hidden)]
pub use spru_script_macro::scriptable;

// /// A [spru::State] with scripting support. Implement using the [tagset::tagset] macro.
// /// ```ignore
// /// #[tagset(impl<Action, Registry> ScriptableState<Action, Registry>)]
// /// ```
// #[telety(crate, alias_traits = "always")]
// #[tagset_meta]
// #[meta(bounds(
//     Registry: self::Registry<Self, Action>,
//     for<VAR> Registry: self::RegistryState<Self, Action, VAR>,
// ))]
// pub trait Scriptable<Action, Registry: ?Sized>: spru::State 
// where
//     Registry: self::Registry<Self, Action>,
// {
//     /// Registers substates with the scripting implementation.
//     /// [ScriptRegistryState::register_state] should be called for each applicable type.
//     #[meta(default {
//         foreach!(VAR => {
//             spru_script::RegistryState::<Self, Action, VAR>::register_state::<Storage>(
//                 registry, 
//                 registration, 
//                 Some(spru_script::scriptable_path!(VAR))
//             )?;
//         });
//         Ok(())
//     })]
//     fn register<Storage>(registry: &Registry, registration: &mut Registry::Registration<'_, Storage>) 
//         -> Result<(), Registry::Error>
//     where
//         Storage: spru::item::Storage<State = Self>,
//     ;
// }
