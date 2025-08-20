use rust_fsm::state_machine;

state_machine! {
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub machine(OptionalPlay)

    OptionalPlay => {
        Play => MandatoryPlay,
        Pass => OptionalPlay,
    },
    MandatoryPlay => {
        Play => MandatoryPlay,
        Score => OptionalPlay,
    },
}
