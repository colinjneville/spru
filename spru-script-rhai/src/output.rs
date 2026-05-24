pub trait Output<Ret> {
    type RetIn;

    fn apply(&self, map: &mut rhai::Map);

    fn triggers(&mut self, map: &mut rhai::Map);

    fn apply_ret(&mut self, ret: Self::RetIn) -> Ret;
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

    fn apply_ret(&mut self, ret: Self::RetIn) -> Ret {
        ret
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

            for arg in rest {
                queue.push(arg.take());
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