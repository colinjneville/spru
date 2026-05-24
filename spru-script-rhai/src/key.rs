use std::sync::LazyLock;

pub const GLOBAL_CONTEXT: LazyLock<rhai::ImmutableString> = LazyLock::new(|| rhai::ImmutableString::from("context"));

pub const GLOBAL_OUTPUT: LazyLock<rhai::ImmutableString> = LazyLock::new(|| rhai::ImmutableString::from("output"));

pub const GLOBAL_ARGS: LazyLock<rhai::ImmutableString> = LazyLock::new(|| rhai::ImmutableString::from("args"));

pub const OUTPUT_ENQUEUE_TRIGGER: LazyLock<rhai::ImmutableString> = LazyLock::new(|| rhai::ImmutableString::from("enqueue_trigger"));

pub const OUTPUT_TRIGGER_QUEUE: LazyLock<rhai::ImmutableString> = LazyLock::new(|| rhai::ImmutableString::from("trigger_queue"));

pub const GLOBAL_TYPE: LazyLock<rhai::ImmutableString> = LazyLock::new(|| rhai::ImmutableString::from("type"));
