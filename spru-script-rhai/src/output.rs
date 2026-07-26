use std::any;

pub trait Output<Ret> {
    type RetIn;

    fn apply(&self, map: &mut rhai::Map);

    fn triggers(&mut self, map: &mut rhai::Map);

    fn apply_ret(&mut self, ret: Self::RetIn) -> Result<Ret, rhai::EvalAltResult>;
}

impl<Ret> Output<Ret> for spru::game::init::Output {
    type RetIn = Ret;

    fn apply(&self, _map: &mut rhai::Map) {
        
    }

    fn triggers(&mut self, _map: &mut rhai::Map) {
        
    }

    fn apply_ret(&mut self, ret: Self::RetIn) -> Result<Ret, rhai::EvalAltResult> {
        Ok(ret)
    }
}

impl<Ret> Output<Ret> for spru::player::init::Output {
    type RetIn = Ret;

    fn apply(&self, _map: &mut rhai::Map) {
        
    }

    fn triggers(&mut self, _map: &mut rhai::Map) {
        
    }

    fn apply_ret(&mut self, ret: Self::RetIn) -> Result<Ret, rhai::EvalAltResult> {
        Ok(ret)
    }
}

impl<Trigger, Ret> Output<Ret> for spru::interaction::Output<Trigger>
where
    Trigger: Clone + Send + Sync + 'static,
{
    type RetIn = Ret;

    fn apply(&self, map: &mut rhai::Map) {
        insert_enqueue_trigger(map);
    }

    fn triggers(&mut self, map: &mut rhai::Map) {
        apply_enqueued_triggers(map, |trigger| <Self as spru::interactor::EnqueueTrigger>::enqueue_trigger(self, trigger));
    }

    fn apply_ret(&mut self, ret: Self::RetIn) -> Result<Ret, rhai::EvalAltResult> {
        Ok(ret)
    }
}

impl<Trigger, GameOutcome> Output<()> for spru::reaction::Output<Trigger, GameOutcome>
where
    Trigger: Clone + Send + Sync + 'static,
    GameOutcome: Clone + Send + Sync + 'static,
{
    type RetIn = rhai::Dynamic;

    fn apply(&self, map: &mut rhai::Map) {
        insert_enqueue_trigger(map);
        // insert_end_game(map);
    }

    fn triggers(&mut self, map: &mut rhai::Map) {
        apply_enqueued_triggers(map, |trigger| <Self as spru::interactor::EnqueueTrigger>::enqueue_trigger(self, trigger));
        // apply_game_outcome(map, |game_outcome| <Self as spru::interactor::SetGameOutcome>::set_game_outcome(self, game_outcome));
    }

    fn apply_ret(&mut self, ret: Self::RetIn) -> Result<(), rhai::EvalAltResult> {
        use spru::interactor::SetGameOutcome as _;

        if !ret.is_unit() {
            let actual = ret.type_name();
            let go = ret.try_cast::<GameOutcome>()
                .ok_or_else(|| {
                    let actual = actual.to_string();
                    let expected = any::type_name::<GameOutcome>().to_string();
                    rhai::EvalAltResult::ErrorMismatchOutputType(expected, actual, rhai::Position::NONE)
                })?;

            self.set_game_outcome(go);
        }

        Ok(())
    }
}

#[allow(deprecated)]
fn insert_enqueue_trigger(map: &mut rhai::Map) {
    let enqueue_trigger = rhai::FnPtr::from_fn(
        &*crate::key::OUTPUT_ENQUEUE_TRIGGER,
        |_ctx: rhai::NativeCallContext<'_>, args: &mut [&mut rhai::Dynamic]| {
            let (first, rest) = args.split_first_mut()
                .ok_or("Must be called as a method")?;

            let mut map = first.as_map_mut()?;

            let queue = map.get_mut(&**crate::key::OUTPUT_TRIGGER_QUEUE)
                .ok_or("Trigger queue missing")?;

            let mut queue = queue.as_array_mut()?;

            println!("pushing {} triggers", rest.len());
            for arg in rest {
                queue.push(arg.take());
                println!("has {} triggers", queue.len());
            }

            Ok(rhai::Dynamic::UNIT)
        }
    ).unwrap();
    map.insert(crate::key::OUTPUT_TRIGGER_QUEUE.clone().into(), rhai::Dynamic::from_array(vec![]));
    map.insert(crate::key::OUTPUT_ENQUEUE_TRIGGER.clone().into(), enqueue_trigger.into());
}

fn apply_enqueued_triggers<Trigger>(map: &mut rhai::Map, mut f: impl FnMut(Trigger)) 
where 
    Trigger: Clone + Send + Sync + 'static,
{
    let mut trigger_queue = map.remove(&**crate::key::OUTPUT_TRIGGER_QUEUE)
        .expect("Trigger queue missing");
    let trigger_queue = trigger_queue.as_array_mut()
        .expect("Expected an array");

    for trigger in &*trigger_queue {
        let trigger = trigger.clone().cast::<Trigger>();
        f(trigger);
    }
}

// #[allow(deprecated)]
// fn insert_end_game(map: &mut rhai::Map) {
//     let end_game = rhai::FnPtr::from_fn(
//         &*crate::key::OUTPUT_END_GAME,
//         |ctx: rhai::NativeCallContext<'_>, args: &mut [&mut rhai::Dynamic]| {
//             let (first, rest) = args.split_first_mut()
//                 .ok_or("Must be called as a method")?;

//             let mut map = first.as_map_mut()?;

//             let (game_outcome, rest) = rest.split_first_mut()
//                 .ok_or("Expected a game outcome")?;

//             crate::dynamic::extra_args_error(&ctx, rest)?;

//             map.insert((*crate::key::OUTPUT_GAME_OUTCOME).clone().into(), game_outcome.take());

//             Ok(rhai::Dynamic::UNIT)
//         }
//     ).unwrap();

//     map.insert(crate::key::OUTPUT_END_GAME.clone().into(), end_game.into());
// }

// fn apply_game_outcome<GameOutcome>(map: &mut rhai::Map, mut f: impl FnMut(GameOutcome)) 
// where 
//     GameOutcome: Clone + Send + Sync + 'static,
// {
//     let game_outcome = map.remove(&**crate::key::OUTPUT_GAME_OUTCOME);
    
//     if let Some(game_outcome) = game_outcome {
//         f(game_outcome.cast());
//     }
// }