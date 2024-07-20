use std::fmt;

use crate::{cards::Cards, rank::Rank, suite::Suite};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card(i8);

impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank(), self.suite())
    }
}

impl Card {
    pub fn of(rank: Rank, suite: Suite) -> Self {
        Self(suite.to_index() + rank.to_i8())
    }

    pub fn from_index(index: i8) -> Option<Self> {
        if index < 0 || index > 63 {
            None
        } else if Cards::MASK_FULL&(1u64 << u64::try_from(index).unwrap()) == 0 {
            None
        } else {
            Some(Self(index))
        }      
    }

    pub fn all() -> impl Iterator<Item = Self> {
        Suite::SUITES.iter()
            .flat_map(|suite| Rank::RANKS.iter().map(|rank| Self::of(*rank, *suite)))
    }

    pub fn rank(self) -> Rank {
        Rank::try_from(self.0 % 16).unwrap()
    }

    pub fn suite(self) -> Suite {
        Suite::try_from(self.0 / 16).unwrap()
    }

    pub fn to_index(self) -> i8 {
        self.0
    }

    pub fn to_index_u64(self) -> u64 {
        self.to_index() as u64
    }
}
