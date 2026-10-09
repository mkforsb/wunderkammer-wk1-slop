//! Wunderkammer WK-1: a two-channel cabinet of improbable sound objects.
//!
//! * `dx serve --platform web` (feature `web`): WebAudio AudioWorklet.
//! * `dx serve --platform desktop` (feature `desktop`): native Linux window,
//!   PulseAudio output.

mod audio;
mod state;
mod ui;

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
        dioxus::LaunchBuilder::desktop()
            .with_cfg(
                Config::new()
                    .with_menu(None)
                    .with_background_color((16, 12, 11, 255))
                    .with_window(
                        WindowBuilder::new()
                            .with_title("Wunderkammer WK-1")
                            .with_inner_size(LogicalSize::new(1400.0, 900.0))
                            .with_min_inner_size(LogicalSize::new(720.0, 600.0)),
                    ),
            )
            .launch(ui::App);
    }

    #[cfg(not(feature = "desktop"))]
    dioxus::launch(ui::App);
}
