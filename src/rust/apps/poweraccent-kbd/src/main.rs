//! PowerAccent keyboard hook service.
//!
//! Installs a global `WH_KEYBOARD_LL` hook, feeds events into the
//! [`poweraccent_core::state_machine::AccentStateMachine`], and uses `SendInput`
//! to emit the selected accent character.

#[cfg(windows)]
mod hook;

fn main() {
    #[cfg(windows)]
    {
        hook::run();
    }

    #[cfg(not(windows))]
    {
        eprintln!("poweraccent-kbd only runs on Windows");
        std::process::exit(1);
    }
}
