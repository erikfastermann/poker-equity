use std::{cmp::Ordering, fmt};

use crate::card::Card;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hand(Card, Card);

impl fmt::Display for Hand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.high(), self.low())
    }
}

impl fmt::Debug for Hand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl Hand {
    pub fn of_cards(a: Card, b: Card) -> Self {
        match a.rank().cmp(&b.rank()) {
            Ordering::Less => Self(b, a),
            Ordering::Equal => match a.suite().to_usize().cmp(&b.suite().to_usize()) {
                Ordering::Less => Self(b, a),
                Ordering::Equal => unreachable!(),
                Ordering::Greater => Self(a, b),
            },
            Ordering::Greater => Self(a, b),
        }
    }

    pub fn high(self) -> Card {
        self.0
    }

    pub fn low(self) -> Card {
        self.1
    }

    pub fn suited(self) -> bool {
        self.high().suite() == self.low().suite()
    }

    pub fn cmp_by_rank(self, other: Self) -> Ordering {
        self.high().rank().cmp(&other.high().rank())
            .then_with(|| self.low().rank().cmp(&other.low().rank()))
            .then_with(|| self.high().suite().to_usize().cmp(&other.high().suite().to_usize()))
            .then_with(|| self.low().suite().to_usize().cmp(&other.low().suite().to_usize()))
    }

    pub fn to_card_array(self) -> [Card; 2] {
        [self.high(), self.low()]
    }
}
