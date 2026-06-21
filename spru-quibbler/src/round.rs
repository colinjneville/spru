use rust_fsm::state_machine;

state_machine! {
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
    #[derive(serde::Serialize, serde::Deserialize)]
    machine_internal(OptionalPlay)

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

pub mod machine {
    pub use super::machine_internal::*;

    #[spru_script::script(state = false, derive = [Eq])]
    impl Input {
        #[function]
        fn full_play() -> Self {
            Self::FullPlay
        }

        #[function]
        fn partial_play() -> Self {
            Self::PartialPlay
        }

        #[function]
        fn pass() -> Self {
            Self::Pass
        }

        #[function]
        fn score() -> Self {
            Self::Score
        }
    }
}

impl Clone for machine::Output {
    fn clone(&self) -> Self {
        match *self { }
    }
}
