use rust_fsm::state_machine;
use spru_script::Wrap;

state_machine! {
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub machine(OptionalPlay)

    OptionalPlay => {
        // The first play must use all cards in valid words
        FullPlay => MandatoryPlay,
        Pass => OptionalPlay,
    },
    MandatoryPlay => {
        PartialPlay => MandatoryPlay,
        FullPlay => MandatoryPlay,
        Score => OptionalPlay,
    },
}


#[spru_script::script(state = false, derive = [Eq])]
impl machine::Input {
    #[function]
    fn full_play() -> Wrap<Self> {
        Wrap::new(Self::FullPlay)
    }

    #[function]
    fn partial_play() -> Wrap<Self> {
        Wrap::new(Self::PartialPlay)
    }

    #[function]
    fn pass() -> Wrap<Self> {
        Wrap::new(Self::Pass)
    }

    #[function]
    fn score() -> Wrap<Self> {
        Wrap::new(Self::Score)
    }
}

impl Clone for machine::Output {
    fn clone(&self) -> Self {
        match *self { }
    }
}
