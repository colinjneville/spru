pub struct Registration<'r> {
    pub(crate) rhai: &'r mut rhai::Engine,
    pub(crate) globals: rhai::Map,
    pub(crate) static_maps: rhai::Map,
}

impl<'r> Registration<'r> {
    pub(crate) fn new(rhai: &'r mut rhai::Engine) -> Self {
        Self {
            rhai,
            globals: rhai::Map::new(),
            static_maps: rhai::Map::new(),
        }
    }
}

pub struct RegistrationState<'r> {
    pub(crate) rhai: &'r mut rhai::Engine,
    pub(crate) map: rhai::Map,
}

impl<'r> RegistrationState<'r> {
    pub(crate) fn new(rhai: &'r mut rhai::Engine) -> Self {
        Self {
            rhai,
            map: rhai::Map::new(),
        }
    }
}

pub struct RegistrationType<'r> {
    pub(crate) rhai: &'r mut rhai::Engine,
    pub(crate) map: rhai::Map,
}

impl<'r> RegistrationType<'r>
{
    pub(crate) fn new(rhai: &'r mut rhai::Engine) -> Self {
        Self {
            rhai,
            map: rhai::Map::new(),
        }
    }
}