use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) enum BackgroundMode {
    #[default]
    Checkerboard,
    Dark,
    Light,
}

impl BackgroundMode {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Checkerboard => Self::Dark,
            Self::Dark => Self::Light,
            Self::Light => Self::Checkerboard,
        }
    }
}

impl FromStr for BackgroundMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "checkerboard" => Ok(Self::Checkerboard),
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            _ => Err("background must be checkerboard, dark, or light".into()),
        }
    }
}

impl fmt::Display for BackgroundMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Checkerboard => "checkerboard",
            Self::Dark => "dark",
            Self::Light => "light",
        })
    }
}

pub(crate) const DEFAULT_THUMBNAIL_SIZE: ThumbnailSize = ThumbnailSize {
    width: 32,
    height: 14,
};
pub(crate) const DEFAULT_THUMBNAIL_QUALITY: ThumbnailQuality = ThumbnailQuality(7);

const MIN_WIDTH: u16 = 12;
const MIN_HEIGHT: u16 = 6;
const MAX_WIDTH: u16 = 120;
const MAX_HEIGHT: u16 = 60;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ThumbnailQuality(u8);

impl ThumbnailQuality {
    pub(crate) const fn level(self) -> u8 {
        self.0
    }

    pub(crate) fn from_digit(character: char) -> Option<Self> {
        character
            .to_digit(10)
            .and_then(|level| Self::new(level as u8).ok())
    }

    fn new(level: u8) -> Result<Self, String> {
        if (1..=9).contains(&level) {
            Ok(Self(level))
        } else {
            Err("thumbnail quality must be a level from 1 to 9".into())
        }
    }
}

impl FromStr for ThumbnailQuality {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse::<u8>()
            .map_err(|_| "thumbnail quality must be a level from 1 to 9".to_owned())
            .and_then(Self::new)
    }
}

impl fmt::Display for ThumbnailQuality {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ThumbnailSize {
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl ThumbnailSize {
    pub(crate) fn larger_for_grid(
        self,
        area_width: u16,
        area_height: u16,
        columns: usize,
        rows: usize,
    ) -> Self {
        let width = if columns > 1 {
            area_width / (columns as u16 - 1)
        } else {
            self.width
        };
        let height = if rows > 1 {
            area_height / (rows as u16 - 1)
        } else {
            self.height
        };
        Self {
            width: width.clamp(MIN_WIDTH, MAX_WIDTH),
            height: height.clamp(MIN_HEIGHT, MAX_HEIGHT),
        }
    }

    pub(crate) fn smaller_for_grid(
        self,
        area_width: u16,
        area_height: u16,
        columns: usize,
        rows: usize,
    ) -> Self {
        let width = area_width / (columns as u16 + 1);
        let height = area_height / (rows as u16 + 1);
        Self {
            width: width.clamp(MIN_WIDTH, MAX_WIDTH),
            height: height.clamp(MIN_HEIGHT, MAX_HEIGHT),
        }
    }
}

impl FromStr for ThumbnailSize {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((width, height)) = value.split_once(['x', 'X', '×']) else {
            return Err("thumbnail size must use WIDTHxHEIGHT, for example 32x14".into());
        };
        let width = width
            .parse::<u16>()
            .map_err(|_| "thumbnail width must be a positive integer")?;
        let height = height
            .parse::<u16>()
            .map_err(|_| "thumbnail height must be a positive integer")?;

        if !(MIN_WIDTH..=MAX_WIDTH).contains(&width) || !(MIN_HEIGHT..=MAX_HEIGHT).contains(&height)
        {
            return Err(format!(
                "thumbnail size must be between {MIN_WIDTH}x{MIN_HEIGHT} and {MAX_WIDTH}x{MAX_HEIGHT} terminal cells"
            ));
        }

        Ok(Self { width, height })
    }
}

impl fmt::Display for ThumbnailSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}x{}", self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ascii_and_typographic_dimensions() {
        assert_eq!(
            "40x18".parse(),
            Ok(ThumbnailSize {
                width: 40,
                height: 18
            })
        );
        assert_eq!(
            "40X18".parse(),
            Ok(ThumbnailSize {
                width: 40,
                height: 18
            })
        );
        assert_eq!(
            "40×18".parse(),
            Ok(ThumbnailSize {
                width: 40,
                height: 18
            })
        );
    }

    #[test]
    fn rejects_malformed_and_unbounded_dimensions() {
        assert!("wide".parse::<ThumbnailSize>().is_err());
        assert!("11x6".parse::<ThumbnailSize>().is_err());
        assert!("32x61".parse::<ThumbnailSize>().is_err());
    }

    #[test]
    fn interactive_adjustment_stays_bounded() {
        let minimum = ThumbnailSize {
            width: 12,
            height: 6,
        };
        let maximum = ThumbnailSize {
            width: 120,
            height: 60,
        };
        assert_eq!(minimum.smaller_for_grid(12, 6, 1, 1), minimum);
        assert_eq!(maximum.larger_for_grid(120, 60, 1, 1), maximum);
        assert_eq!(
            DEFAULT_THUMBNAIL_SIZE
                .larger_for_grid(80, 20, 2, 1)
                .to_string(),
            "80x14"
        );
        assert_eq!(
            DEFAULT_THUMBNAIL_SIZE
                .smaller_for_grid(80, 20, 2, 1)
                .to_string(),
            "26x10"
        );
    }

    #[test]
    fn quality_accepts_exactly_levels_one_through_nine() {
        for level in 1..=9 {
            assert_eq!(level.to_string().parse(), Ok(ThumbnailQuality(level)));
            assert_eq!(
                ThumbnailQuality::from_digit(char::from_digit(level.into(), 10).unwrap()),
                Some(ThumbnailQuality(level))
            );
        }
        assert!("0".parse::<ThumbnailQuality>().is_err());
        assert!("10".parse::<ThumbnailQuality>().is_err());
        assert!("high".parse::<ThumbnailQuality>().is_err());
        assert_eq!(ThumbnailQuality::from_digit('0'), None);
    }
}
