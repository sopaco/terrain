//! Shared `ts-rs` attributes for IPC types exported to the Svelte frontend.

/// Wrap a struct/enum item for optional TypeScript export.
#[macro_export]
macro_rules! ts_ipc {
    (
        $(#[$meta:meta])*
        $vis:vis $kind:ident $name:ident $($rest:tt)*
    ) => {
        #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
        #[cfg_attr(
            feature = "ts-export",
            ts(export)
        )]
        $(#[$meta])*
        $vis $kind $name $($rest)*
    };
}
