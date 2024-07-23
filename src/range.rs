use core::fmt;
use std::cmp::{max, min};

use crate::card::Card;
use crate::cards::CardsByRank;
use crate::rank::Rank;
use crate::result::Result;

#[derive(Clone, Copy)]
struct RangeEntry {
    high: Rank,
    low: Rank,
    suited: bool,
}

impl fmt::Display for RangeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.high, self.low)?;
        if self.high == self.low {
            write!(f, "-")
        } else if self.suited {
            write!(f, "s")
        } else {
            write!(f, "o")
        }
    }
}

impl RangeEntry {
    fn first_second(self) -> (Rank, Rank) {
        debug_assert!(self.high >= self.low);
        if self.suited {
            (self.high, self.low)
        } else {
            (self.low, self.high)
        }
    }
}

#[derive(Clone)]
pub struct RangeTable {
    table: [CardsByRank; Rank::COUNT],
}

impl fmt::Display for RangeTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in Rank::RANKS.iter().rev().copied() {
            let mut iter = Rank::RANKS.iter().rev().copied().peekable();
            while let Some(column) = iter.next() {
                let entry = RangeEntry {
                    high: max(row, column),
                    low: min(row, column),
                    suited: column < row,
                };
                let contains = if self.contains_entry(entry) {
                    "T"
                } else {
                    "F"
                };
                write!(f, "{} ({})", entry, contains)?;
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
    pub fn empty() -> Self {
        Self { table: [CardsByRank::EMPTY; Rank::COUNT] }
    }

    pub fn full() -> Self {
        let mut range = Self::empty();
        for row in Rank::RANKS.iter().rev().copied() {
            for column in Rank::RANKS.iter().rev().copied() {
                let high = max(row, column);
                let low = min(row, column);
                let suited = column < row;
                range.add(RangeEntry { high, low, suited });
            }
        }
        range
    }

    pub fn parse(range_str: &str) -> Result<Self> {
        let mut range = Self::empty();
        for def in range_str.split(',') {
            let result = match def.as_bytes() {
                [pair_a, pair_b] if pair_a == pair_b => range.parse_pair(*pair_a),
                [pair_a, pair_b, b'+'] if pair_a == pair_b => range.parse_pairs_asc(*pair_a),
                [high, low, b'o'] => range.parse_one(*high, *low, false),
                [high, low, b'o', b'+'] => range.parse_asc(*high, *low, false),
                [high, low, b's'] => range.parse_one(*high, *low, true),
                [high, low, b's', b'+'] => range.parse_asc(*high, *low, true),
                _ => Err("parsing failed".into()),
            };

            if let Err(err) = result {
                return Err(format!(
                    "invalid range '{}': invalid entry '{}': {}",
                    range_str,
                    def,
                    err,
                ).into())
            }
        }

        Ok(range)
    }

    fn contains_entry(&self, entry: RangeEntry) -> bool {
        let (a, b) = entry.first_second();
        self.table[a.to_usize()].has(b)
    }

    fn add(&mut self, entry: RangeEntry) {
        let (a, b) = entry.first_second();
        self.table[a.to_usize()].add(b)
    }

    fn try_add(&mut self, entry: RangeEntry) -> Result<()> {
        let (a, b) = entry.first_second();
        if self.table[a.to_usize()].try_add(b) {
            Ok(())
        } else {
            Err(format!("range table add failed: duplicate entry {}", entry).into())
        }
    }

    pub fn contains(&self, a: Card, b: Card) -> bool {
        let high = max(a.rank(), b.rank());
        let low = min(a.rank(), b.rank());
        let suited = a.suite() == b.suite();
        let entry = RangeEntry { high, low, suited };
        self.contains_entry(entry)
    }

    fn parse_pair(&mut self, raw_rank: u8) -> Result<()> {
        let rank = Rank::from_ascii(raw_rank)?;
        self.try_add(RangeEntry { high: rank, low: rank, suited: false })?;
        Ok(())
    }

    fn parse_pairs_asc(&mut self, raw_rank: u8) -> Result<()> {
        let from = Rank::from_ascii(raw_rank)?;
        for rank in Rank::range(from, Rank::Ace) {
            let entry = RangeEntry { high: rank, low: rank, suited: false };
            self.try_add(entry)?;
        }
        Ok(())
    }

    fn parse_one(&mut self, raw_high: u8, raw_low: u8, suited: bool) -> Result<()> {
        let high = Rank::from_ascii(raw_high)?;
        let low = Rank::from_ascii(raw_low)?;
        if low >= high {
            Err("low greater or equals to high".into())
        } else {
            self.try_add(RangeEntry { high, low, suited })
        }
    }

    fn parse_asc(&mut self, raw_high: u8, raw_low: u8, suited: bool) -> Result<()> {
        let high = Rank::from_ascii(raw_high)?;
        let low = Rank::from_ascii(raw_low)?;
        if low >= high {
            return Err("low greater or equals to high".into());
        }
        for rank in Rank::range(low, high.predecessor().unwrap()) {
            self.try_add(RangeEntry { high, low: rank, suited })?;
        }
        Ok(())
    }
}
