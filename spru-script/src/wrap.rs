use std::marker::PhantomData;

use crate::marker;


#[derive(Debug)]
#[repr(transparent)]
pub struct Wrap<Marker, Args>(Option<Args>, PhantomData<Marker>);

impl<Marker, Args> Wrap<Marker, Args> {
    pub fn new(args: Args) -> Self {
        Self(Some(args), PhantomData)
    }

    pub fn take(&mut self) -> Args {
        self.0.take()
            .expect("take called only once")
    }
}


macro_rules! wrap_constructors {
    ($($marker:ty => $constructor:ident),* $(,)?) => {
        $(
            impl<Args> Wrap<$marker, Args> {
                pub fn $constructor(args: Args) -> Self {
                    Self::new(args)
                }
            }
        )*
    }
}

wrap_constructors! {
    marker::State => new_state,
    (marker::State, marker::Get) => new_state_get,
    (marker::State, marker::Set) => new_state_set,
    (marker::State, marker::Method) => new_state_method,
    (marker::State, marker::Function) => new_state_function,
    (marker::State, marker::Create) => new_state_create,
    marker::Stateless => new_type,
    (marker::Stateless, marker::Get) => new_stateless_get,
    (marker::Stateless, marker::Method) => new_stateless_method,
    (marker::Stateless, marker::Function) => new_stateless_function,
    (marker::Stateless, marker::Eq) => new_stateless_eq,
}

pub type StatelessArgs<T> = (PhantomData<(T, )>, );
pub type StatelessWrap<T> = Wrap<marker::Stateless, StatelessArgs<T>>;

pub type StatelessEqArgs<T> = (PhantomData<(T, )>, );
pub type StatelessEqWrap<T> = Wrap<(marker::Stateless, marker::Eq), StatelessEqArgs<T>>;

pub type StatelessGetArgs<'n, T, U> = (&'n str, fn(&T) -> U, PhantomData<()>);
pub type StatelessGetWrap<'n, T, U> = Wrap<(marker::Stateless, marker::Get), StatelessGetArgs<'n, T, U>>;

pub type StatelessMethodArgs<'n, T, Args, Ret> = (&'n str, fn(&T, Args) -> Ret, PhantomData<()>);
pub type StatelessMethodWrap<'n, T, Args, Ret> = Wrap<(marker::Stateless, marker::Method), StatelessMethodArgs<'n, T, Args, Ret>>;

pub type StatelessFunctionArgs<'n, Args, Ret> = (&'n str, fn(Args) -> Ret, PhantomData<()>);
pub type StatelessFunctionWrap<'n, Args, Ret> = Wrap<(marker::Stateless, marker::Function), StatelessFunctionArgs<'n, Args, Ret>>;

pub type StateArgs<Action, T> = (PhantomData<(Action, T)>, );
pub type StateWrap<Action, T> = Wrap<marker::State, StateArgs<Action, T>>;

pub type StateGetArgs<'n, Action, T, U> = (&'n str, fn(&T) -> U, PhantomData<Action>);
pub type StateGetWrap<'n, Action, T, U> = Wrap<(marker::State, marker::Get), StateGetArgs<'n, Action, T, U>>;

pub type StateSetArgs<'n, Action, T, U, Ret> = (&'n str, fn(&T, U) -> Ret, PhantomData<Action>);
pub type StateSetWrap<'n, Action, T, U, Ret> = Wrap<(marker::State, marker::Set), StateSetArgs<'n, Action, T, U, Ret>>;

pub type StateMethodArgs<'n, Action, T, Args, Ret> = (&'n str, fn(&T, Args) -> Ret, PhantomData<Action>);
pub type StateMethodWrap<'n, Action, T, Args, Ret> = Wrap<(marker::State, marker::Method), StateMethodArgs<'n, Action, T, Args, Ret>>;

pub type StateFunctionArgs<'n, Action, Args, Ret> = (&'n str, fn(Args) -> Ret, PhantomData<Action>);
pub type StateFunctionWrap<'n, Action, Args, Ret> = Wrap<(marker::State, marker::Function), StateFunctionArgs<'n, Action, Args, Ret>>;

pub type StateCreateArgs<'n, Action, T, Args, Create> = (&'n str, fn(Args) -> Create, PhantomData<(Action, T)>);
pub type StateCreateWrap<'n, Action, T, Args, Create> = Wrap<(marker::State, marker::Create), StateCreateArgs<'n, Action, T, Args, Create>>;
