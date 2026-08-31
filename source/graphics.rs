use std::{env, str::FromStr};

use crossterm::terminal::{size, window_size};
use ratatui_image::{
    FontSize,
    picker::{Picker, ProtocolType},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum GraphicsProtocol {
    #[default]
    Auto,
    Kitty,
    Sixel,
    Iterm2,
    Halfblocks,
}

impl FromStr for GraphicsProtocol {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "kitty" => Ok(Self::Kitty),
            "sixel" => Ok(Self::Sixel),
            "iterm2" => Ok(Self::Iterm2),
            "halfblocks" => Ok(Self::Halfblocks),
            _ => Err("graphics protocol must be auto, kitty, sixel, iterm2, or halfblocks".into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionSource {
    Auto,
    Environment,
    Forced,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct TerminalEnvironment {
    direct_kitty: bool,
}

impl TerminalEnvironment {
    fn from_process() -> Self {
        let in_tmux = nonempty_variable("TMUX");
        let kitty_hint = nonempty_variable("KITTY_WINDOW_ID")
            || env::var("TERM").is_ok_and(|term| term.to_ascii_lowercase().contains("kitty"));
        Self {
            direct_kitty: kitty_hint && !in_tmux,
        }
    }
}

fn nonempty_variable(name: &str) -> bool {
    env::var_os(name).is_some_and(|value| !value.is_empty())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Selection {
    protocol: ProtocolType,
    source: SelectionSource,
}

pub(crate) fn select_picker(requested: GraphicsProtocol, query_terminal: bool) -> (Picker, String) {
    let mut picker = if query_terminal {
        Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
    } else {
        queryless_picker()
    };
    let selection = select(
        requested,
        picker.protocol_type(),
        TerminalEnvironment::from_process(),
    );
    picker.set_protocol_type(selection.protocol);
    let label = selection.label();
    (picker, label)
}

fn queryless_picker() -> Picker {
    let font_size = size()
        .ok()
        .zip(window_size().ok())
        .and_then(|((columns, rows), pixels)| {
            (columns > 0 && rows > 0 && pixels.width > 0 && pixels.height > 0).then(|| {
                FontSize::new(
                    (pixels.width / columns).max(1),
                    (pixels.height / rows).max(1),
                )
            })
        })
        .unwrap_or(FontSize::new(10, 20));
    #[allow(deprecated)]
    Picker::from_fontsize(font_size)
}

fn select(
    requested: GraphicsProtocol,
    queried: ProtocolType,
    environment: TerminalEnvironment,
) -> Selection {
    let forced = match requested {
        GraphicsProtocol::Auto => None,
        GraphicsProtocol::Kitty => Some(ProtocolType::Kitty),
        GraphicsProtocol::Sixel => Some(ProtocolType::Sixel),
        GraphicsProtocol::Iterm2 => Some(ProtocolType::Iterm2),
        GraphicsProtocol::Halfblocks => Some(ProtocolType::Halfblocks),
    };
    if let Some(protocol) = forced {
        return Selection {
            protocol,
            source: SelectionSource::Forced,
        };
    }
    if queried == ProtocolType::Halfblocks && environment.direct_kitty {
        return Selection {
            protocol: ProtocolType::Kitty,
            source: SelectionSource::Environment,
        };
    }
    Selection {
        protocol: queried,
        source: SelectionSource::Auto,
    }
}

impl Selection {
    fn label(self) -> String {
        let protocol = match self.protocol {
            ProtocolType::Halfblocks => "Half 1x2",
            ProtocolType::Sixel => "Sixel",
            ProtocolType::Kitty => "Kitty",
            ProtocolType::Iterm2 => "iTerm2",
        };
        let source = match self.source {
            SelectionSource::Auto => "auto",
            SelectionSource::Environment => "env",
            SelectionSource::Forced => "forced",
        };
        format!("{protocol}/{source}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_HINTS: TerminalEnvironment = TerminalEnvironment {
        direct_kitty: false,
    };
    const DIRECT_KITTY: TerminalEnvironment = TerminalEnvironment { direct_kitty: true };

    #[test]
    fn accepts_exactly_the_documented_protocol_names() {
        for value in ["auto", "kitty", "sixel", "iterm2", "halfblocks"] {
            assert!(value.parse::<GraphicsProtocol>().is_ok());
        }
        assert!("half".parse::<GraphicsProtocol>().is_err());
        assert!("Kitty".parse::<GraphicsProtocol>().is_err());
    }

    #[test]
    fn active_true_pixel_query_remains_authoritative() {
        assert_eq!(
            select(GraphicsProtocol::Auto, ProtocolType::Sixel, DIRECT_KITTY),
            Selection {
                protocol: ProtocolType::Sixel,
                source: SelectionSource::Auto,
            }
        );
    }

    #[test]
    fn direct_kitty_hint_recovers_from_halfblocks_fallback() {
        assert_eq!(
            select(
                GraphicsProtocol::Auto,
                ProtocolType::Halfblocks,
                DIRECT_KITTY
            ),
            Selection {
                protocol: ProtocolType::Kitty,
                source: SelectionSource::Environment,
            }
        );
    }

    #[test]
    fn halfblocks_remains_the_safe_default_without_a_hint() {
        assert_eq!(
            select(GraphicsProtocol::Auto, ProtocolType::Halfblocks, NO_HINTS),
            Selection {
                protocol: ProtocolType::Halfblocks,
                source: SelectionSource::Auto,
            }
        );
    }

    #[test]
    fn explicit_override_wins_over_query_and_environment() {
        let selected = select(
            GraphicsProtocol::Halfblocks,
            ProtocolType::Kitty,
            DIRECT_KITTY,
        );
        assert_eq!(selected.protocol, ProtocolType::Halfblocks);
        assert_eq!(selected.source, SelectionSource::Forced);
        assert_eq!(selected.label(), "Half 1x2/forced");
    }

    #[test]
    fn queryless_picker_has_nonzero_sample_geometry() {
        let picker = queryless_picker();
        assert!(picker.font_size().width > 0);
        assert!(picker.font_size().height > 0);
    }
}
