use std::cmp::Ordering;

use crate::{card::Card, cards::{Cards, Top5}, range::RangeTable};

#[derive(Debug, Clone, Copy)]
pub struct Equity {
    won: u32,
    chop: u32,
    total: u32,
}

impl Equity {
    fn zero() -> Self {
        Self {
            won: 0,
            chop: 0,
            total: 0,
        }
    }

    pub fn equity(
        community_cards: Cards,
        hero_cards: Cards,
        villain_ranges: &[impl AsRef<RangeTable>],
    ) -> Equity {
        EquityCalculator::new(community_cards, hero_cards, villain_ranges).calc()
    }

    pub fn equity_percent(self) -> f64 {
        f64::from(self.won + self.chop) / f64::from(self.total)
    }

    pub fn win_percent(self) -> f64 {
        f64::from(self.won) / f64::from(self.total)
    }

    pub fn tie_percent(self) -> f64 {
        f64::from(self.chop) / f64::from(self.total)
    }
}

struct EquityCalculator<'a, RT: AsRef<RangeTable>> {
    equity: Equity,
    hero_top5: Top5,
    known_cards: Cards,
    community_cards: Cards,
    villain_ranges: &'a [RT],
    villain_top5: Vec<Top5>,
}

impl <'a, RT: AsRef<RangeTable>> EquityCalculator<'a, RT> {
    fn new(
        community_cards: Cards,
        hero_cards: Cards,
        villain_ranges: &'a [RT],
    ) -> Self {
        assert!(hero_cards.count() == 2);
        assert!(community_cards.count() >= 3 && community_cards.count() <= 5);
        let known_cards = community_cards | hero_cards;
        assert!(known_cards.count() == community_cards.count()+hero_cards.count());
        assert!(!villain_ranges.is_empty());
        Self {
            equity: Equity::zero(),
            hero_top5: Top5::worst(),
            known_cards,
            community_cards,
            villain_ranges,
            villain_top5: vec![Top5::worst(); villain_ranges.len()],
        }
    }

    fn calc(&mut self) -> Equity {
        assert!(self.equity.total == 0);
        let remaining_community_cards = 5 - self.community_cards.count();
        self.community_cards(remaining_community_cards.into());
        self.equity
    }

    fn community_cards(&mut self, remainder: usize) {
        if remainder == 0 {
            self.hero_top5 = self.known_cards.top5();
            self.players(self.villain_ranges.len() - 1);
            return;
        }

        let current_known_cards = self.known_cards;
        let current_community_cards = self.community_cards;
        for card in Card::all() {
            if current_known_cards.has(card) {
                continue;
            }
            self.known_cards = current_known_cards.with(card);
            self.community_cards = current_community_cards.with(card);
            self.community_cards(remainder - 1);
        }
    }

    fn players(&mut self, remainder: usize) {
        let villain = self.villain_ranges[remainder].as_ref();
        let current_known_cards = self.known_cards;
        for card_a in Card::all() {
            if current_known_cards.has(card_a) {
                continue;
            }
            for card_b in Card::all() {
                if current_known_cards.has(card_b) {
                    continue;
                }
                if card_a == card_b {
                    continue;
                }
                if !villain.contains(card_a, card_b) {
                    continue;
                }
                self.villain_top5[remainder] = self.community_cards
                    .with(card_a)
                    .with(card_b)
                    .top5();
                self.known_cards = current_known_cards.with(card_a).with(card_b);

                if remainder != 0 {
                    self.players(remainder - 1);
                } else {
                    self.showdown();
                }
            }
        }
    }

    fn showdown(&mut self) {
        let (mut loss, mut tie) = (0usize, 0usize);
        for villain_top5 in self.villain_top5.iter().copied() {
            match self.hero_top5.compare(villain_top5) {
                Ordering::Less => loss += 1,
                Ordering::Equal => tie += 1,
                Ordering::Greater => (),
            }
        }
        self.equity.total += 1;
        if loss == 0 {
            if tie == 0 {
                self.equity.won += 1;
            } else {
                self.equity.chop += 1;
            }
        }
    }
}
