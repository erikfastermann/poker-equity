use std::cmp::Ordering;

use crate::card::Card;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hand(Card, Card);

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
}
