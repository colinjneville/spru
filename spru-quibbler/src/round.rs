use rust_fsm::state_machine;

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
