pub trait Context {
    fn apply(&self, map: &mut rhai::Map);
}

impl<Root> Context for spru::interaction::Context<'_, Root> 
where
    Root: Clone + Send + Sync + 'static,
{
    fn apply(&self, map: &mut rhai::Map) {
        map.insert("root".into(), rhai::Dynamic::from(self.root.clone()));
        map.insert("player".into(), rhai::Dynamic::from(self.player.clone()));
    }
}

impl<Root> Context for spru::reaction::Context<'_, Root> 
where
    Root: Clone + Send + Sync + 'static,
{
    fn apply(&self, map: &mut rhai::Map) {
        map.insert("root".into(), rhai::Dynamic::from(self.root.clone()));
        
        if let Some(player) = &self.player {
            map.insert("player".into(), rhai::Dynamic::from(player.clone()));
        }
    }
}

impl Context for spru::game::init::Context {
    fn apply(&self, _map: &mut rhai::Map) {
        
    }
}

impl<Root> Context for spru::player::init::Context<'_, Root> 
where
    Root: Clone + Send + Sync + 'static,
{
    fn apply(&self, map: &mut rhai::Map) {
        map.insert("root".into(), rhai::Dynamic::from(self.root.clone()));
        map.insert("player".into(), rhai::Dynamic::from(self.player.clone()));
    }
}
