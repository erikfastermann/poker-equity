use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};

use crate::{card::Card, cards::Cards, range::{RangeSimulator, RangeTable}};

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

fn check_input(
    community_cards: Cards,
    hero_cards: Cards,
    villain_ranges: &[impl AsRef<RangeTable>],
) {
    assert!(hero_cards.count() == 2);
    assert!(community_cards.count() <= 5);
    let known_cards = community_cards | hero_cards;
    assert!(known_cards.count() == community_cards.count()+hero_cards.count());
    assert!(!villain_ranges.is_empty());
    assert!(villain_ranges.len() <= 8);
}

fn random_card<R: Rng>(rng: &mut R, known_cards: &mut Cards) -> Card {
    for _ in 0..1000 { // TODO
        let card = rng.r#gen();
        if !known_cards.has(card) {
            known_cards.add(card);
            return card;
        }
    }
    panic!()
}

impl Equity {
    fn from_total_wins_ties(total: u64, wins: &[u64], ties: &[f64]) -> Vec<Self> {
        assert_eq!(wins.len(), ties.len());
        let mut equities = Vec::with_capacity(wins.len());
        for (wins, ties) in wins.iter().copied().zip(ties.iter().copied()) {
            equities.push(Equity { wins, ties, total });
        }
        equities
    }

    pub fn calc(
        community_cards: Cards,
        hero_cards: Cards,
        villain_ranges: &[impl AsRef<RangeTable>],
    ) -> Vec<Equity> {
        EquityCalculator::new(community_cards, hero_cards, villain_ranges).calc()
    }

    pub fn simulate(
        start_community_cards: Cards,
        hero_cards: Cards,
        villain_ranges: &[impl AsRef<RangeTable>],
        rounds: usize,
    ) -> Vec<Equity> {
        // TODO: This seams biased towards hero.

        check_input(start_community_cards, hero_cards, villain_ranges);
        let remaining_community_cards = 5 - start_community_cards.count();
        let mut rng = SmallRng::from_entropy();

        let mut range_simulators = {
            let (a, b) = hero_cards.to_hand().unwrap();
            let mut range_simulators = vec![RangeSimulator::of_hand(a, b)];
            range_simulators.extend(villain_ranges.iter()
                .map(|range| range.as_ref().to_range_simulator(&mut rng)));
            range_simulators
        };

        let mut total = 0u64;
        let mut scores = vec![0u32; range_simulators.len()];
        let mut wins = vec![0u64; range_simulators.len()];
        let mut ties = vec![0.0; range_simulators.len()];
        let mut indices: Vec<_> = (0..range_simulators.len()).collect();

        for _ in 0..rounds {
            for _ in 0..2 {
                let community_cards = {
                    let mut community_cards = start_community_cards;
                    for _ in 0..remaining_community_cards {
                        random_card(&mut rng, &mut community_cards);
                    }
                    community_cards
                };

                indices.shuffle(&mut rng);
                let mut valid_hand = true;
                let mut known_cards = community_cards;
                for i in indices.iter().copied() {
                    let range = &mut range_simulators[i];
                    let Some((a, b)) = range.random_hand(&mut rng, &mut known_cards) else {
                        valid_hand = false;
                        break;
                    };
                    scores[i] = community_cards.with(a).with(b).top5().to_score();
                }

                if valid_hand {
                    total += 1;
                    showdown(&scores, &mut wins, &mut ties);
                    break;
                }
            }
        }

        Self::from_total_wins_ties(total, &wins, &ties)
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
        check_input(community_cards, hero_cards, villain_ranges);
        Self {
            known_cards: community_cards | hero_cards,
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
        Equity::from_total_wins_ties(self.total, &self.wins, &self.ties)
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
        showdown(&self.hand_ranking_scores, &mut self.wins, &mut self.ties)
    }
}

fn showdown(
    hand_ranking_scores: &[u32],
    wins: &mut [u64],
    ties: &mut [f64],
) {
    let max_score = hand_ranking_scores.iter().copied().max().unwrap();
    let winners = hand_ranking_scores.iter()
        .copied()
        .filter(|score| *score == max_score)
        .count();
    if winners == 1 {
        let winner_index = hand_ranking_scores.iter()
            .position(|score| *score == max_score)
            .unwrap();
        wins[winner_index] += 1;
    } else {
        let ratio = 1.0 / try_u64_to_f64(u64::try_from(winners).unwrap()).unwrap();
        for (index, score) in hand_ranking_scores.iter().copied().enumerate() {
            if score == max_score {
                ties[index] += ratio;
            }
        }
    }
}
