use core::fmt;
use std::cmp::{max, min};
use std::collections::HashSet;

use crate::card::Card;
use crate::rank::Rank;
use crate::result::Result;

pub struct RangeTable {
    set: HashSet<RangeEntry>,
}

impl fmt::Display for RangeTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in Rank::RANKS.iter().rev().copied() {
            let mut iter = Rank::RANKS.iter().rev().copied().peekable();
            while let Some(column) = iter.next() {
                let high = max(row, column);
                let low = min(row, column);
                write!(f, "{}{}", high, low)?;
                let suited = if column < row {
                    write!(f, "s")?;
                    true
                } else if column == row {
                    write!(f, "-")?;
                    false
                } else {
                    write!(f, "o")?;
                    false
                };
                let contains = if self.set.contains(&RangeEntry{ high, low, suited }) {
                    "T"
                } else {
                    "F"
                };
                write!(f, "({})", contains)?;
                if iter.peek().is_some() {
                    write!(f, " ")?;
                }
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

impl RangeTable {
    pub fn contains(&self, a: Card, b: Card) -> bool {
        let high = max(a.rank(), b.rank());
        let low = min(a.rank(), b.rank());
        let suited = a.suite() == b.suite();
        self.set.contains(&RangeEntry { high, low, suited })
    }

    pub fn full() -> RangeTable {
        let mut set = HashSet::new();
        for row in Rank::RANKS.iter().rev().copied() {
            for column in Rank::RANKS.iter().rev().copied() {
                let high = max(row, column);
                let low = min(row, column);
                let suited = column < row;
                set.insert(RangeEntry{ high, low, suited });
            }
        }
        RangeTable { set }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct RangeEntry {
    pub(crate) high: Rank,
    pub(crate) low: Rank,
    pub(crate) suited: bool,
}

impl fmt::Display for RangeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = if self.suited { "s" } else { "o" };
        write!(f, "{}{}{}", self.high, self.low, suffix)
    }
}

impl RangeEntry {
    fn pair(raw_rank: u8) -> Option<Vec<Self>> {
        let rank = Rank::from_ascii(raw_rank)?;
        Some(vec![Self { high: rank, low: rank, suited: false }])
    }

    fn pairs_asc(raw_rank: u8) -> Option<Vec<Self>> {
        let from = Rank::from_ascii(raw_rank)?;
        let ranks = Rank::range(from, Rank::Ace)
            .map(|rank| Self { high: rank, low: rank, suited: false })
            .collect();
        Some(ranks)
    }

    fn one(raw_high: u8, raw_low: u8, suited: bool) -> Option<Vec<Self>> {
        let high = Rank::from_ascii(raw_high)?;
        let low = Rank::from_ascii(raw_low)?;
        if low >= high {
            return None;
        }
        Some(vec![Self { high, low, suited }])
    }

    fn asc(raw_high: u8, raw_low: u8, suited: bool) -> Option<Vec<Self>> {
        let high = Rank::from_ascii(raw_high)?;
        let low = Rank::from_ascii(raw_low)?;
        if low >= high {
            return None;
        }
        let ranks = Rank::range(low, high.predecessor().unwrap())
            .map(|rank| Self { high, low: rank, suited})
            .collect();
        Some(ranks)
    }
}

pub fn parse_range(range: &str) -> Result<RangeTable> {
    let mut out = HashSet::new();

    for def in range.split_ascii_whitespace() {
        let entries = match def.as_bytes() {
            [pair_a, pair_b] if pair_a == pair_b => RangeEntry::pair(*pair_a),
            [pair_a, pair_b, b'+'] if pair_a == pair_b => RangeEntry::pairs_asc(*pair_a),
            [high, low, b'o'] => RangeEntry::one(*high, *low, false),
            [high, low, b'o', b'+'] => RangeEntry::asc(*high, *low, false),
            [high, low, b's'] => RangeEntry::one(*high, *low, true),
            [high, low, b's', b'+'] => RangeEntry::asc(*high, *low, true),
            _ => None,
        };

        match entries {
            Some(entries) => out.extend(entries),
            None => return Err(format!("invalid range '{}': invalid entry '{}'", range, def).into()),
        };
    }

    Ok(RangeTable { set: out })
}
