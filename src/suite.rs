use std::fmt;

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suite {
    Diamonds = 0,
    Spades = 1,
    Hearts = 2,
    Clubs = 3,
}

use Suite::*;

impl fmt::Display for Suite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suite = match *self {
            Diamonds => "d",
            Spades => "s",
            Hearts => "h",
            Clubs => "c",
        };
        write!(f, "{}", suite)
    }
}

impl TryFrom<i8> for Suite {
    type Error = ();

    fn try_from(v: i8) -> std::result::Result<Self, Self::Error> {
        match v {
            x if x == Diamonds as i8 => Ok(Diamonds),
            x if x == Spades as i8 => Ok(Spades),
            x if x == Hearts as i8 => Ok(Hearts),
            x if x == Clubs as i8 => Ok(Clubs),
            _ => Err(()),
        }
    }
}

impl Suite {
    pub const SUITES: [Suite; 4] = [
        Diamonds,
        Spades,
        Hearts,
        Clubs,
    ];

    fn underlying(self) -> i8 {
        self as i8
    }

    pub fn to_index(self) -> i8 {
        let index = self.underlying() * 16;
        debug_assert!(index >= 0 && index < 64);
        index
    }

    pub fn to_index_u64(self) -> u64 {
        self.to_index() as u64
    }
}
