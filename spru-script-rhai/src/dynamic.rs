use std::any;

pub trait IntoDynamic: 'static {
    fn into_dynamic(self) -> rhai::Dynamic;
}

impl<T> IntoDynamic for Option<T> 
where
    T: Clone + Send + Sync + 'static,
{
    fn into_dynamic(self) -> rhai::Dynamic {
        match self {
            Some(some) => rhai::Dynamic::from(some),
            None => rhai::Dynamic::UNIT,
        }
    }
}

impl<T> IntoDynamic for Vec<T> 
where
    T: Clone + Send + Sync + 'static,
{
    fn into_dynamic(self) -> rhai::Dynamic {
        let mut array = rhai::Array::new();
        for e in self {
            array.push(rhai::Dynamic::from(e));
        }
        rhai::Dynamic::from_array(array)
    }
}

crate::expand_foreach! { $
    { impl IntoDynamic for } [i8, i16, i32, i64, isize, u8, u16, u32, u64, usize] { {
        fn into_dynamic(self) -> rhai::Dynamic {
            // TODO this could panic on u64/usize/isize -> i64 (or u32/i64 if INT is i32)
            rhai::Dynamic::from_int(self as rhai::INT)
        }
    } }
}


pub trait FromDynamic: Sized + 'static {
    fn from_dynamic(ctx: &rhai::NativeCallContext<'_>, dynamic: rhai::Dynamic) -> Result<Self, Box<rhai::EvalAltResult>>;
}

impl<T> FromDynamic for Option<T>
where 
    T: Clone + Send + Sync + 'static,
{
    fn from_dynamic(ctx: &rhai::NativeCallContext<'_>, dynamic: rhai::Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
        if dynamic.is_unit() {
            return Ok(None);
        }

        let dynamic = match dynamic.try_cast_result() {
            Ok(some) => return Ok(Some(some)),
            Err(dynamic) => dynamic,
        };

        let dynamic = match dynamic.try_cast_result() {
            Ok(some) => return Ok(some),
            Err(dynamic) => dynamic,
        };

        Err(Box::new(rhai::EvalAltResult::ErrorMismatchDataType(any::type_name::<Option<T>>().to_string(), dynamic.type_name().to_string(), ctx.call_position())))
    }
}

impl<T> FromDynamic for Vec<T>
where 
    T: Clone + Send + Sync + 'static,
{
    fn from_dynamic(ctx: &rhai::NativeCallContext<'_>, dynamic: rhai::Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
        let dynamic = match dynamic.try_cast_result() {
            Ok(v) => return Ok(v),
            Err(dynamic) => dynamic,
        };

        dynamic.into_typed_array()
            .map_err(|ty| rhai::EvalAltResult::ErrorMismatchDataType(
                any::type_name::<T>().to_string(), 
                ty.to_string(), 
                ctx.call_position(),
            ))
            .map_err(Box::new)
    }
}

crate::expand_foreach! { $
    { impl FromDynamic for } [i8, i16, i32, i64, isize, u8, u16, u32, u64, usize] { {
        fn from_dynamic(ctx: &rhai::NativeCallContext<'_>, dynamic: rhai::Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
            let dynamic = match dynamic.try_cast_result() {
                Ok(value) => return Ok(value),
                Err(dynamic) => dynamic,
            };

            let i = dynamic.as_int()
                .map_err(|ty| rhai::EvalAltResult::ErrorMismatchDataType(
                    any::type_name::<Self>().to_string(), 
                    ty.to_string(), 
                    ctx.call_position(),
                ))?;

            i.try_into()
                .map_err(|_| rhai::EvalAltResult::ErrorRuntime(rhai::Dynamic::from(format!("INT value '{i}' too large for {}", any::type_name::<Self>())), ctx.call_position()))
                .map_err(Box::new)
        }
    } }
}

pub(crate) fn from_array<T>(ctx: rhai::NativeCallContext<'_>, mut args: &mut [&mut rhai::Dynamic]) 
    -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>> 
where
    T: Clone + Send + Sync + 'static,
{
    // This will be called from a map, so ignore the `this` arguments
    let _ = args.split_off_first_mut();
    let array = pop_type::<rhai::Array>(&ctx, &mut args)?;
    extra_args_error(&ctx, &mut args)?;

    let mut v = Vec::with_capacity(array.len());
    for element in array {
        match element.try_cast_result::<T>() {
            Ok(t) => {
                v.push(t);
            }
            Err(element) => {
                return Err(Box::new(rhai::EvalAltResult::ErrorMismatchDataType(
                    std::any::type_name::<T>().to_string(), 
                    element.type_name().to_string(), 
                    ctx.call_position()
                )));
            }
        }
    }
    Ok(rhai::Dynamic::from(v))
}

pub(crate) fn flatten<T>(option: &mut Option<T>) -> rhai::Dynamic 
where 
    T: Clone + Send + Sync + 'static,
{
    match option {
        Some(some) => rhai::Dynamic::from(some.clone()),
        None => rhai::Dynamic::UNIT,
    }
}

pub(crate) fn to_array<T>(v: &mut Vec<T>) -> rhai::Array 
where
    T: Clone + Send + Sync + 'static,
{
    v.iter()
        .cloned()
        .map(rhai::Dynamic::from)
        .collect::<rhai::Array>()
}

pub(crate) fn pop_type<T>(ctx: &rhai::NativeCallContext<'_>, args: &mut &mut [&mut rhai::Dynamic]) -> Result<T, rhai::EvalAltResult> 
where
    T: Clone + Sync + Send + 'static,
{
    if let Some(arg) = args.split_off_first_mut() {
        let arg = arg.take();
        match arg.try_cast_result::<T>() {
            Ok(arg) => {
                Ok(arg)
            }
            Err(arg) => {
                let expected = std::any::type_name::<T>().to_string();
                let actual = arg.type_name().to_string();
                Err(rhai::EvalAltResult::ErrorMismatchDataType(expected, actual, ctx.call_position()))
            }
        }
    } else {
        Err(rhai::EvalAltResult::ErrorRuntime("Not enough arguments".into(), ctx.call_position()))
    }
}

pub(crate) fn extra_args_error(ctx: &rhai::NativeCallContext<'_>, args: &[&mut rhai::Dynamic]) -> crate::RhaiResult<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(rhai::EvalAltResult::ErrorRuntime("Too many arguments".into(), ctx.call_position()))
    }
}

