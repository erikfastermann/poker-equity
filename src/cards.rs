use std::{cmp::Ordering, fmt, ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, Shl}};

use crate::{card::Card, rank::Rank, suite::Suite};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRanking {
    HighCard,
    OnePair(Rank),
    TwoPair { first: Rank, second: Rank },
    ThreeOfAKind(Rank),
    Straight,
    Flush,
    FullHouse { trips: Rank, pair: Rank },
    FourOfAKind(Rank),
    StraightFlush,
    RoyalFlush,
}

#[derive(Debug, Clone, Copy)]
pub struct Top5 {
    ranking: HandRanking,
    cards: Cards,
}

impl Top5 {
    fn of(ranking: HandRanking, cards: Cards) -> Self {
        debug_assert!(cards.count() <= 5);
        Self { ranking, cards }
    }

    pub fn compare(self, villain: Top5) -> Ordering {
        match self.ranking.cmp(&villain.ranking) {
            Ordering::Equal => {
                let hero_rankings = self.cards.by_rank();
                let villain_rankings = villain.cards.by_rank();
                let iter = hero_rankings.iter().zip(villain_rankings.iter());
                for (hero_rank, villain_rank) in iter {
                    match hero_rank.cmp(&villain_rank) {
                        Ordering::Equal => continue,
                        o => return o,
                    }
                }
                Ordering::Equal
            },
            o => o,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Cards(u64);

impl fmt::Display for Cards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut cards = self.iter().peekable();
        write!(f, "[")?;
        while let Some(card) = cards.next() {
            write!(f, "{card}")?;
            if cards.peek().is_some() {
                write!(f, " ")?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl fmt::Debug for Cards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl BitAnd<Cards> for Cards {
    type Output = Cards;

    fn bitand(self, rhs: Cards) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitOr<Cards> for Cards {
    type Output = Cards;

    fn bitor(self, rhs: Cards) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign<Cards> for Cards {
    fn bitor_assign(&mut self, rhs: Cards) {
        self.0 |= rhs.0;
    }
}

impl Not for Cards {
    type Output = Cards;

    fn not(self) -> Self::Output {
        Self((!self.0) & Self::MASK_FULL)
    }
}

impl Cards {
    pub const EMPTY: Self = Cards(0);

    pub const MASK_SINGLE: u64 = 0b1_1111_1111_1111;

    pub const MASK_FULL: u64 = Cards::MASK_SINGLE << 48
        | Cards::MASK_SINGLE << 32
        | Cards::MASK_SINGLE << 16
        | Cards::MASK_SINGLE;

    pub fn from_slice(s: &[Card]) -> Option<Self> {
        let mut cards = Self::EMPTY;
        for card in s.iter().copied() {
            if cards.has(card) {
                return None;
            }
            cards.add(card);
        }
        Some(cards)
    }

    fn of_rank(rank: Rank) -> Self {
        Cards::from_slice(&[
            Card::of(rank, Suite::Diamonds),
            Card::of(rank, Suite::Spades),
            Card::of(rank, Suite::Hearts),
            Card::of(rank, Suite::Clubs),
        ]).unwrap()
    }

    fn to_u64(self) -> u64 {
        self.0
    }

    pub fn first(self) -> Option<Card> {
        let index = 63 - self.0.leading_zeros() as i8;
        Card::from_index(index)
    }

    pub fn has(self, card: Card) -> bool {
        (self.0 & (1 << card.to_index_u64())) != 0
    }

    pub fn add(&mut self, card: Card) {
        assert!(!self.has(card));
        self.0 |= 1 << card.to_index_u64();
    }

    pub fn with(self, card: Card) -> Self {
        assert!(!self.has(card));
        Self(self.0 | (1 << card.to_index_u64()))
    }

    pub fn remove(&mut self, card: Card) {
        assert!(self.has(card));
        self.0 &= !(1 << card.to_index_u64())
    }

    fn without_rank(self, rank: Rank) -> Self {
        self & !Self::of_rank(rank)
    }

    pub fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    fn by_rank(self) -> CardsByRank {
        CardsByRank::from_cards(self)
    }

    fn take_n(self, n: u8) -> Self {
        let mut out = Self::EMPTY;
        for card in self.iter().take(n.into()) {
            out.add(card);
        }
        out
    }

    fn suites(self) -> impl Iterator<Item = (Suite, CardsByRank)> {
        Suite::SUITES.iter()
            .copied()
            .map(move |suite| (suite, CardsByRank::from_cards_suite(self, suite)))
    }

    pub fn top5(self) -> Top5 {
        let counts = self.counts();
        if let Some(cards) = self.straight_flush() {
            if cards.first().unwrap().rank() == Rank::Ace {
                Top5::of(HandRanking::RoyalFlush, cards)
            } else {
                Top5::of(HandRanking::StraightFlush, cards)
            }
        } else if let Some((rank, cards)) = self.quads(counts) {
            Top5::of(HandRanking::FourOfAKind(rank), cards)
        } else if let Some((trips, pair, cards)) = self.full_house(counts) {
            Top5::of(HandRanking::FullHouse { trips, pair }, cards)
        } else if let Some(cards) = self.flush() {
            Top5::of(HandRanking::Flush, cards)
        } else if let Some(cards) = self.straight() {
            Top5::of(HandRanking::Straight, cards)
        } else if let Some((rank, cards)) = self.trips(counts) {
            Top5::of(HandRanking::ThreeOfAKind(rank), cards)
        } else if let Some(top5) = self.pair(counts) {
            top5
        } else {
            Top5::of(HandRanking::HighCard, self.kickers(5))
        }
    }

    fn kickers(self, count: u8) -> Self {
        let mut kickers = Self::EMPTY;
        let mut remaining = count;
        for rank in self.by_rank().iter() {
            let next = (self & Self::of_rank(rank)).take_n(remaining);
            remaining -= next.count();
            kickers |= next;
            if remaining == 0 {
                return kickers;
            }
        }
        kickers
    }

    fn pair(self, counts: [u8; Rank::COUNT]) -> Option<Top5> {
        let Some(first_pair_rank) = Self::best_n(counts, 2) else {
            return None;
        };
        let first_pair = (self & Cards::of_rank(first_pair_rank)).take_n(2);

        let second_pair_rank = {
            let mut counts_without_first_pair = counts;
            counts_without_first_pair[first_pair_rank.to_usize()] = 0;
            match Self::best_n(counts_without_first_pair, 2) {
                Some(rank) => rank,
                None => {
                    let kickers = self.without_rank(first_pair_rank).kickers(3);
                    let cards = first_pair | kickers;
                    return Some(Top5::of(HandRanking::OnePair(first_pair_rank), cards));
                },
            }
        };
        let second_pair = (self & Cards::of_rank(second_pair_rank)).take_n(2);

        let kicker = self.without_rank(first_pair_rank)
            .without_rank(second_pair_rank)
            .kickers(1);
        let cards = first_pair | second_pair | kicker;
        let ranking = HandRanking::TwoPair {
            first: first_pair_rank,
            second: second_pair_rank,
        };
        Some(Top5::of(ranking, cards))
    }

    fn trips(self, counts: [u8; Rank::COUNT]) -> Option<(Rank, Self)> {
        if let Some(trips_rank) = Self::best_n(counts, 3) {
            let trips = (self & Cards::of_rank(trips_rank)).take_n(3);
            let kickers = self.without_rank(trips_rank).kickers(2);
            Some((trips_rank, trips|kickers))
        } else {
            None
        }
    }

    fn straight(self) -> Option<Self> {
        let Some(straight) = self.by_rank().straight() else {
            return None;
        };
        let mut out = Self::EMPTY;
        for rank in straight.iter() {
            out |= (self & Self::of_rank(rank)).take_n(1);
        }
        Some(out)
    }

    fn flush(self) -> Option<Self> {
        let mut flush = None;
        for (suite, cards) in self.suites() {
            if cards.count() >= 5 {
                assert!(flush.is_none());
                flush = Some(cards.take_top_n(5).to_cards_suite(suite));
            }
        }
        flush
    }

    fn full_house(self, counts: [u8; Rank::COUNT]) -> Option<(Rank, Rank, Self)> {
        let Some(trips_rank) = Self::best_n(counts, 3) else {
            return None;
        };
        let pair_rank = {
            let mut counts_without_trips = counts;
            counts_without_trips[trips_rank.to_usize()] = 0;
            match Self::best_n(counts_without_trips, 2) {
                Some(rank) => rank,
                None => return None,
            }
        };
        let trips = (self & Cards::of_rank(trips_rank)).take_n(3);
        let pair = (self & Cards::of_rank(pair_rank)).take_n(2);
        Some((trips_rank, pair_rank, trips|pair))
    }

    fn quads(self, counts: [u8; Rank::COUNT]) -> Option<(Rank, Self)> {
        if let Some(quads_rank) = Self::best_n(counts, 4) {
            let mut quads = Cards::of_rank(quads_rank);
            let mut kicker_cards = self.by_rank();
            kicker_cards.remove(quads_rank);
            if let Some(kicker) = kicker_cards.highest_rank() {
                quads |= (self & Self::of_rank(kicker)).take_n(1);
            }
            Some((quads_rank, quads))
        } else {
            None
        }
    }

    fn best_n(counts: [u8; Rank::COUNT], n: u8) -> Option<Rank> {
        let mut best_index = None;
        for (index, count) in counts.iter().copied().enumerate() {
            if count >= n {
                let index = i8::try_from(index).unwrap();
                best_index = Some(index);
            }
        }
        best_index.map(|index| Rank::try_from(index).unwrap())
    }

    fn counts(self) -> [u8; Rank::COUNT] {
        let mut counts = [0; Rank::COUNT];
        for card in self.iter() {
            counts[card.rank().to_usize()] += 1;
        }
        counts
    }

    fn straight_flush(self) -> Option<Self> {
        let mut straight_flush = None;
        for (suite, cards) in self.suites() {
            if let Some(straight) = cards.straight() {
                assert!(straight_flush.is_none());
                straight_flush = Some(straight.to_cards_suite(suite));
            }
        }
        straight_flush
    }

    pub fn iter(self) -> CardsIter {
        CardsIter(self)
    }
}

pub struct CardsIter(Cards);

impl Iterator for CardsIter {
    type Item = Card;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.first() {
            Some(card) => {
                self.0.remove(card);
                Some(card)
            },
            None => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CardsByRank(i16);

impl fmt::Display for CardsByRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ranks = self.iter().peekable();
        write!(f, "[")?;
        while let Some(rank) = ranks.next() {
            write!(f, "{rank}")?;
            if ranks.peek().is_some() {
                write!(f, " ")?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl BitAnd<CardsByRank> for CardsByRank {
    type Output = CardsByRank;

    fn bitand(self, rhs: CardsByRank) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign<CardsByRank> for CardsByRank {
    fn bitand_assign(&mut self, rhs: CardsByRank) {
        self.0 &= rhs.0;
    }
}

impl BitOr<CardsByRank> for CardsByRank {
    type Output = CardsByRank;

    fn bitor(self, rhs: CardsByRank) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign<CardsByRank> for CardsByRank {
    fn bitor_assign(&mut self, rhs: CardsByRank) {
        self.0 |= rhs.0;
    }
}

impl Shl<i8> for CardsByRank {
    type Output = CardsByRank;

    fn shl(self, rhs: i8) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl CardsByRank {
    const EMPTY: Self = CardsByRank(0);

    const WHEEL: Self = Self(0b1_0000_0000_1111);
    const STRAIGHT_SIX_HIGH: Self = Self(0b11111);

    fn from_cards(cards: Cards) -> Self {
        let n = cards.to_u64();
        let collapsed = (n | (n >> 16) | (n >> 32) | (n >> 48)) & Cards::MASK_SINGLE;
        CardsByRank(collapsed as i16)
    }

    fn from_cards_suite(cards: Cards, suite: Suite) -> CardsByRank {
        let rank = (cards.to_u64() >> suite.to_index()) & Cards::MASK_SINGLE;
        CardsByRank(rank as i16)
    }

    fn to_cards_suite(self, suite: Suite) -> Cards {
        Cards((self.0 as u64) << suite.to_index_u64())
    }

    fn highest_rank(self) -> Option<Rank> {
        Rank::try_from(15 - self.0.leading_zeros() as i8).ok()
    }

    fn has(self, rank: Rank) -> bool {
        (self.0 & (1 << rank.to_i16())) != 0
    }

    pub fn add(&mut self, rank: Rank) {
        assert!(!self.has(rank));
        self.0 |= 1 << rank.to_i16();
    }

    fn remove(&mut self, rank: Rank) {
        assert!(self.has(rank));
        self.0 &= !(1 << rank.to_i16());
    }

    fn without(mut self, rank: Rank) -> Self {
        assert!(self.has(rank));
        Self(self.0 & !(1 << rank.to_i16()))
    }

    fn iter(self) -> CardsByRankIter {
        CardsByRankIter(self)
    }

    fn straight(self) -> Option<Self> {
        let mut best_cards = None;
        if self&Self::WHEEL == Self::WHEEL {
            best_cards = Some(Self::WHEEL);
        }
        for shift in 0..=13-5 {
            let straight = Self::STRAIGHT_SIX_HIGH << shift;
            if self&straight == straight {
                best_cards = Some(straight);
            }
        }
        best_cards
    }

    fn count(self) -> i8 {
        self.0.count_ones() as i8
    }

    fn take_top_n(self, n: u8) -> Self {
        let mut out = Self::EMPTY;
        for rank in self.iter().take(n.into()) {
            out.add(rank);
        }
        out
    }
}

struct CardsByRankIter(CardsByRank);

impl Iterator for CardsByRankIter {
    type Item = Rank;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.highest_rank() {
            Some(rank) => {
                self.0.remove(rank);
                Some(rank)
            },
            None => None,
        }
    }
}
