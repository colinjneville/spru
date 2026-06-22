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

pub use spru_script_base::{LanguageActual, StatelessDialect, StatelessDialectEval, Dialect, DialectExec, DialectEval, StatelessLexicon, Lexicon};

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
