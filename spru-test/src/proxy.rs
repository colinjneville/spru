pub mod std {
    pub mod fmt {
        use std::fmt;

        #[telety::telety(crate::proxy::std::fmt, proxy = "fmt::Debug")]
        pub trait Debug {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), std::fmt::Error>;
        }
    }
}
