pub trait StatelessLexicon {
    type Language: crate::LanguageActual;

    /// Register non-State immutable types
    fn register_stateless(registration: &mut <Self::Language as crate::LanguageActual>::Registration<'_>);
}

pub trait Lexicon: StatelessLexicon {
    type Action: spru::Action;

    /// Register State types contained by this Lexicon for use in [Self::Language].  
    fn register_state<Storage>(registration: &mut <Self::Language as crate::LanguageActual>::Registration<'_>)
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    ;
}

pub trait LanguageActual {
    type Registration<'r>;
    // type Registered<Lexicon: crate::StatelessLexicon>;
    // type Error;

    // fn with_stateless_lexicon<Lexicon: crate::StatelessLexicon>() 
    //     -> Result<Self::Registered<Lexicon>, Self::Error>;
    // fn with_lexicon<Lexicon: crate::Lexicon>() 
    //     -> Result<Self::Registered<Lexicon>, Self::Error>;
}

pub trait StatelessLanguage {
    // type Registration<'r>;
    type Error;
}

pub trait Language: StatelessLanguage {
    type Action: spru::Action;
}

pub trait LanguageStatelessEval<Args, Ret>: StatelessLanguage {
    // Evaluate the result of a script without Storage access
    fn stateless_eval(&self, script: &str, args: Args) -> Result<Ret, Self::Error>;
}

pub trait LanguageEval<Args, Ret, Root>: Language + LanguageStatelessEval<Args, Ret> {
    // Evaluate the result of a script with read-only Storage access. Any attempts to modify game items will fail.
    fn eval<Storage>(&self, storage: &Storage, root: &Root, script: &str, args: Args) 
        -> Result<Ret, Self::Error>
    where 
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    ;
}

pub trait LanguageExec<Args, Ret, Context, Output>: Language {
    fn exec<Storage>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Self::Action, Context, Output>, 
        script: &str,
        args: Args,
    ) 
        -> Result<Ret, Self::Error>
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    ;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptablePath(pub &'static [&'static str], pub &'static [Self]);

mod private {
    pub trait Sealed { }
}

/// Used by [script] to allow returning multiple sub-Action types from methods.
pub trait MethodReturn<Action> : private::Sealed {
    type T;

    fn convert(self) -> (Self::T, Vec<Action>);
}

macro_rules! tuple_method_return {
    () => { };
    ($n:tt $first:ident $($nn:tt $rest:ident)*) => {
        impl<T, $first, $($rest),*> private::Sealed for (T, $first, $($rest),*) { }

        impl<T, Action, $first, $($rest),*> MethodReturn<Action> for (T, $first, $($rest),*) 
        where
            $first: Into<Action>,
            $($rest: Into<Action>),*
        {
            type T = T;

            fn convert(self) -> (Self::T, Vec<Action>) {
                let mut v = vec![
                    self.$n.into(),
                    $(self.$nn.into()),*
                ];
                
                v.reverse();

                (self.0, v)
            }
        }
        tuple_method_return!($($nn $rest)*);
    };
}

impl<T> private::Sealed for (T, ) { }

impl<T, Action> MethodReturn<Action> for (T, ) {
    type T = T;

    fn convert(self) -> (Self::T, Vec<Action>) {
        (self.0, vec![])
    }
}

tuple_method_return!(16 P 15 O 14 N 13 M 12 L 11 K 10 J 9 I 8 H 7 G 6 F 5 E 4 D 3 C 2 B 1 A);

/// Used by [script] to allow returning multiple sub-Action types from setters.
pub trait SetReturn<Action> : private::Sealed {
    fn convert(self) -> Vec<Action>;
}

macro_rules! tuple_set_return {
    () => { };
    ($n:tt $first:ident $($nn:tt $rest:ident)*) => {
        // Handled by MethodReturn
        // impl<$first, $($rest),*> private::Sealed for ($first, $($rest),*) { }

        impl<Action, $first, $($rest),*> SetReturn<Action> for ($first, $($rest),*) 
        where
            $first: Into<Action>,
            $($rest: Into<Action>),*
        {
            fn convert(self) -> Vec<Action> {
                let mut v = vec![
                    self.$n.into(),
                    $(self.$nn.into()),*
                ];
                
                v.reverse();

                v
            }
        }
        tuple_set_return!($($nn $rest)*);
    };
}

tuple_set_return!(15 P 14 O 13 N 12 M 11 L 10 K 9 J 8 I 7 H 6 G 5 F 4 E 3 D 2 C 1 B 0 A);