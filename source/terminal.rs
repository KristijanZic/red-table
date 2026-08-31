use std::{
    fs::OpenOptions,
    io::{self, Write, stdout},
    sync::atomic::{AtomicU8, Ordering},
};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub(crate) type AppTerminal = Terminal<CrosstermBackend<Box<dyn Write + Send>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalTarget {
    Stdout,
    Tty,
}

impl TerminalTarget {
    pub(crate) const fn for_result_mode(result_mode: bool) -> Self {
        if result_mode { Self::Tty } else { Self::Stdout }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Stdout => 0,
            Self::Tty => 1,
        }
    }

    const fn from_code(code: u8) -> Self {
        if code == 1 { Self::Tty } else { Self::Stdout }
    }
}

static ACTIVE_TARGET: AtomicU8 = AtomicU8::new(0);

pub(crate) struct TerminalSession {
    terminal: AppTerminal,
    restored: bool,
}

impl TerminalSession {
    pub(crate) fn new(target: TerminalTarget) -> io::Result<Self> {
        install_panic_restore(target);
        ACTIVE_TARGET.store(target.code(), Ordering::Release);
        enable_raw_mode()?;
        let mut writer = writer(target)?;
        if let Err(error) = execute!(writer, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        let backend = CrosstermBackend::new(writer);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                restore_target(target);
                return Err(error);
            }
        };
        Ok(Self {
            terminal,
            restored: false,
        })
    }

    pub(crate) fn terminal_mut(&mut self) -> &mut AppTerminal {
        &mut self.terminal
    }

    pub(crate) fn restore(&mut self) {
        if self.restored {
            return;
        }
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
        self.restored = true;
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        self.restore();
    }
}

fn writer(target: TerminalTarget) -> io::Result<Box<dyn Write + Send>> {
    match target {
        TerminalTarget::Stdout => Ok(Box::new(stdout())),
        TerminalTarget::Tty => OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .map(|file| Box::new(file) as Box<dyn Write + Send>),
    }
}

fn restore_target(target: TerminalTarget) {
    let _ = disable_raw_mode();
    if let Ok(mut writer) = writer(target) {
        let _ = execute!(writer, LeaveAlternateScreen);
    }
}

fn install_panic_restore(target: TerminalTarget) {
    ACTIVE_TARGET.store(target.code(), Ordering::Release);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_target(TerminalTarget::from_code(
            ACTIVE_TARGET.load(Ordering::Acquire),
        ));
        previous(panic_info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_code_round_trips() {
        for target in [TerminalTarget::Stdout, TerminalTarget::Tty] {
            assert_eq!(TerminalTarget::from_code(target.code()), target);
        }
    }
}
