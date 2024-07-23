use crate::{card::Card, cards::Cards, range::RangeTable};

fn try_u64_to_f64(n: u64) -> Option<f64> {
    const F64_MAX_SAFE_INT: u64 = 2 << 53;
    if (F64_MAX_SAFE_INT-1)&n != n {
        None
    } else {
        Some(n as f64)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Equity {
    wins: u64,
    ties: f64,
    total: u64,
}

impl Equity {
    pub fn calc(
        community_cards: Cards,
        hero_cards: Cards,
        villain_ranges: &[impl AsRef<RangeTable>],
    ) -> Vec<Equity> {
        EquityCalculator::new(community_cards, hero_cards, villain_ranges).calc()
    }

    pub fn equity_percent(self) -> f64 {
        (try_u64_to_f64(self.wins).unwrap() + self.ties)
            / try_u64_to_f64(self.total).unwrap()
    }

    pub fn win_percent(self) -> f64 {
        try_u64_to_f64(self.wins).unwrap() / try_u64_to_f64(self.total).unwrap()
    }

    pub fn tie_percent(self) -> f64 {
        self.ties / try_u64_to_f64(self.total).unwrap()
    }
}

struct EquityCalculator<'a, RT: AsRef<RangeTable>> {
    known_cards: Cards,
    community_cards: Cards,
    villain_ranges: &'a [RT],
    hand_ranking_scores: Vec<u32>,
    total: u64,
    wins: Vec<u64>,
    ties: Vec<f64>,
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
            known_cards,
            community_cards,
            villain_ranges,
            hand_ranking_scores: vec![0; villain_ranges.len() + 1],
            total: 0,
            wins: vec![0; villain_ranges.len() + 1],
            ties: vec![0.0; villain_ranges.len() + 1],
        }
    }

    fn calc(&mut self) -> Vec<Equity> {
        assert!(self.total == 0);
        let remaining_community_cards = 5 - self.community_cards.count();
        self.community_cards(remaining_community_cards.into());
        let mut equities = Vec::with_capacity(self.wins.len());
        for (wins, ties) in self.wins.iter().copied().zip(self.ties.iter().copied()) {
            equities.push(Equity { wins, ties, total: self.total });
        }
        equities

    }

    fn community_cards(&mut self, remainder: usize) {
        if remainder == 0 {
            self.hand_ranking_scores[0] = self.known_cards.top5().to_score();
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
        let player_index = self.villain_ranges.len() - remainder - 1;
        let villain = self.villain_ranges[player_index].as_ref();
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
                self.hand_ranking_scores[player_index+1] = self.community_cards
                    .with(card_a)
                    .with(card_b)
                    .top5()
                    .to_score();
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
        self.total += 1;
        let max_score = self.hand_ranking_scores.iter().copied().max().unwrap();
        let winners = self.hand_ranking_scores.iter()
            .copied()
            .filter(|score| *score == max_score)
            .count();
        if winners == 1 {
            let winner_index = self.hand_ranking_scores.iter()
                .position(|score| *score == max_score)
                .unwrap();
            self.wins[winner_index] += 1;
        } else {
            let ratio = 1.0 / try_u64_to_f64(u64::try_from(winners).unwrap()).unwrap();
            for (index, score) in self.hand_ranking_scores.iter().copied().enumerate() {
                if score == max_score {
                    self.ties[index] += ratio;
                }
            }
        }
    }
}
