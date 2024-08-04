use core::fmt;
use std::cmp::{max, min};
use std::collections::HashSet;

use rand::{Rng, seq::SliceRandom};

use crate::card::Card;
use crate::cards::{Cards, CardsByRank};
use crate::hand::Hand;
use crate::rank::Rank;
use crate::result::Result;
use crate::suite::Suite;

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
    fn from_hand(hand: Hand) -> Self {
        RangeEntry {
            high: hand.high().rank(),
            low: hand.low().rank(),
            suited: hand.suited(),
        }
    }

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

    pub fn for_each_hand(&self, mut f: impl FnMut(Hand)) {
        for row_rank in Rank::RANKS {
            let mut row = self.table[row_rank.to_usize()];
            while let Some(column_rank) = row.highest_rank() {
                row.remove(column_rank);
                let suited = row_rank > column_rank;
                debug_assert!({
                    let entry = RangeEntry {
                        high: max(row_rank, column_rank),
                        low: min(row_rank, column_rank),
                        suited,
                    };
                    self.contains_entry(entry)
                });
                if suited {
                    for suite in Suite::SUITES {
                        let hand = Hand::of_cards(
                            Card::of(row_rank, suite),
                            Card::of(column_rank, suite),
                        );
                        f(hand);
                    }
                } else {
                    for suite_a in Suite::SUITES {
                        for suite_b in Suite::SUITES[suite_a.to_usize()+1..].iter().copied() {
                            let hand = Hand::of_cards(
                                Card::of(row_rank, suite_a),
                                Card::of(column_rank, suite_b),
                            );
                            f(hand);
                            if row_rank != column_rank {
                                let hand = Hand::of_cards(
                                    Card::of(row_rank, suite_b),
                                    Card::of(column_rank, suite_a),
                                );
                                f(hand);
                            }
                        }
                    }
                }
            }
        }
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

    pub fn contains(&self, hand: Hand) -> bool {
        self.contains_entry(RangeEntry::from_hand(hand))
    }

    pub fn is_empty(&self) -> bool {
        self.table.iter().all(|row| *row == CardsByRank::EMPTY)
    }

    pub fn count(&self) -> u8 {
        self.table.iter().map(|row| row.count_u8()).sum()
    }

    pub fn count_cards(&self) -> u32 {
        let mut count = 0u32;
        self.for_each_hand(|_| count += 2);
        count
    }

    pub fn card_set(&self) -> Cards {
        let mut cards = Cards::EMPTY;
        self.for_each_hand(|hand| {
            cards.try_add(hand.high());
            cards.try_add(hand.low());
        });
        cards
    }

    pub fn to_set(&self) -> HashSet<Hand> {
        let mut hands = HashSet::new();
        for high in Rank::RANKS.iter().rev().copied() {
            for low in Rank::RANKS[..=high.to_usize()].iter().rev().copied() {
                for suite_a in Suite::SUITES {
                    for suite_b in Suite::SUITES {
                        let suited = suite_a == suite_b;
                        if suited && high == low {
                            continue;
                        }
                        if !self.contains_entry(RangeEntry { high, low, suited }) {
                            continue;
                        }
                        let hand = Hand::of_cards(
                            Card::of(high, suite_a),
                            Card::of(low, suite_b),
                        );
                        hands.insert(hand);
                    }
                }
            }
        }
        hands
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

pub struct RangeSimulator {
    hands: Vec<(Hand, u8)>,
}

impl RangeSimulator {
    pub fn new() -> Self {
        Self { hands: Vec::new() }
    }

    pub fn add(&mut self, hands: impl IntoIterator<Item = Hand>, index: u8) {
        assert!(self.hands.iter().all(|(_, i)| *i != index));
        for hand in hands {
            self.hands.push((hand, index));
        }
    }

    pub fn shuffle(&mut self, rng: &mut impl Rng) {
        self.hands.shuffle(rng);
    }

    pub fn random_hands(
        &mut self,
        rng: &mut impl Rng,
        mut known_cards: Cards,
        hands: &mut [Option<Hand>],
    ) -> bool {
        for hand in hands.iter_mut() {
            *hand = None;
        }

        let mut remaining_players = hands.len();
        let mut len = self.hands.len();
        while len > 0 {
            let hand_index = rng.gen_range(0..len);
            let (hand, player_index) = self.hands[hand_index];
            let player_index = usize::from(player_index);

            if !hands[player_index].is_some()
                && !known_cards.has(hand.high())
                && !known_cards.has(hand.low()) {
                    hands[player_index] = Some(hand);
                    known_cards.add(hand.high());
                    known_cards.add(hand.low());
                    remaining_players -= 1;
                    if remaining_players == 0 {
                        return true;
                    }
            }

            self.hands.swap(hand_index, len-1);
            len -= 1;
        }

        false
    }
}
