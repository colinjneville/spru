use bevy::prelude;

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[reflect(from_reflect = false)]
#[relationship(relationship_target = Buttons)]
pub struct ButtonFor {
    #[relationship]
    dialog: prelude::Entity,
    text: &'static str,
    #[reflect(ignore)]
    on_press: Option<bevy::ecs::world::CommandQueue>,
}

impl ButtonFor {
    pub fn new(dialog: prelude::Entity, text: &'static str, on_press: bevy::ecs::system::SystemId) -> Self {
        Self {
            dialog,
            text,
            on_press: Some(on_press),
        }
    }

    pub fn text(&self) -> &'static str {
        self.text
    }

    pub fn button_for(&self) -> prelude::Entity {
        self.dialog
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[relationship_target(relationship = ButtonFor, linked_spawn)]
pub struct Buttons(Vec<prelude::Entity>);

impl Buttons {
    pub fn buttons(&self) -> &[prelude::Entity] {
        &self.0
    }
}

#[derive(Debug)]
pub struct Press;

impl prelude::EntityCommand for Press {
    type Out = ();

    fn apply(self, mut entity: prelude::EntityWorldMut) -> Self::Out {
        let Ok(button_for) = entity.get_components::<&ButtonFor>() else {
            prelude::error!("Entity does not have a button to press");
            return;
        };

        if let Some(on_press) = button_for.on_press {
            entity.world_scope(|world| {
                if let Err(err) = world.run_system(on_press) {
                    prelude::error!("{err}");
                }
            });
        }
    }
}

#[derive(Debug)]
pub struct Add {
    text: &'static str,
    on_click: bevy::ecs::system::SystemId,
}

impl Add {
    pub fn new(text: &'static str, on_click: bevy::ecs::system::SystemId) -> Self {
        Self {
            text,
            on_click,
        }
    }
}

impl prelude::EntityCommand for Add {
    type Out = ();

    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> Self::Out {
        let Self {
            text,
            on_click,
        } = self;

        let dialog = entity.id();

        entity
            .insert((
                prelude::ChildOf(dialog),
                ButtonFor::new(dialog, text, on_click),
            ))
            ;
    }
}