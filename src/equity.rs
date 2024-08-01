use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};

use crate::{card::Card, cards::{Cards, Score}, range::{RangeSimulator, RangeTable}, rank::Rank, suite::Suite};

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

impl Equity {
    fn from_total_wins_ties(total: u64, wins: &[u64], ties: &[f64]) -> Vec<Self> {
        assert_ne!(total, 0);
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
    ) -> Option<Vec<Equity>> {
        EquityCalculator::new(community_cards, hero_cards, villain_ranges).calc()
    }

    pub fn simulate(
        start_community_cards: Cards,
        hero_cards: Cards,
        villain_ranges: &[impl AsRef<RangeTable>],
        rounds: usize,
    ) -> Option<Vec<Equity>> {
        // TODO: Valid distribution for ranges.

        check_input(start_community_cards, hero_cards, villain_ranges);
        let remaining_community_cards = 5 - start_community_cards.count();
        let mut rng = SmallRng::from_entropy();
        let mut deck = Deck::new(&mut rng);
        let player_count = villain_ranges.len() + 1;

        let mut range_simulator = {
            let mut range_simulator = RangeSimulator::new();
            let hero_hand = hero_cards.to_hand().unwrap();
            range_simulator.add([hero_hand], 0);
            for (index, range) in villain_ranges.iter().enumerate() {
                range_simulator.add(range.as_ref().to_set(), u8::try_from(index+1).unwrap());
            }
            range_simulator.shuffle(&mut rng);
            range_simulator
        };

        let mut total = 0u64;
        let mut scores = vec![Score::ZERO; player_count];
        let mut wins = vec![0u64; player_count];
        let mut ties = vec![0.0; player_count];
        let mut hands = vec![None; player_count];

        for _ in 0..rounds {
            for _ in 0..2 {
                let community_cards = {
                    let mut community_cards = start_community_cards;
                    for _ in 0..remaining_community_cards {
                        deck.draw(&mut rng, &mut community_cards);
                    }
                    community_cards
                };
                if !range_simulator.random_hands(&mut rng, community_cards, &mut hands) {
                    continue;
                }

                for (i, hand) in hands.iter().enumerate() {
                    let hand = hand.unwrap();
                    let player_cards = community_cards.with(hand.high())
                        .with(hand.low());
                    scores[i] = player_cards.score_fast();
                }

                total += 1;
                showdown(&scores, &mut wins, &mut ties);
                break;
            }
        }

        if usize::try_from(total).unwrap() < rounds/2 {
            None
        } else {
            Some(Self::from_total_wins_ties(total, &wins, &ties))
        }
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
    hero_cards: Cards,
    visited_community_cards: Cards,
    community_cards: Cards,
    villain_ranges: &'a [RT],
    hand_ranking_scores: Vec<Score>,
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
            known_cards: Cards::EMPTY,
            hero_cards,
            community_cards,
            visited_community_cards: community_cards | hero_cards,
            villain_ranges,
            hand_ranking_scores: vec![Score::ZERO; villain_ranges.len() + 1],
            total: 0,
            wins: vec![0; villain_ranges.len() + 1],
            ties: vec![0.0; villain_ranges.len() + 1],
        }
    }

    fn calc(&mut self) -> Option<Vec<Equity>> {
        assert!(self.total == 0);
        let remaining_community_cards = 5 - self.community_cards.count();
        self.community_cards(remaining_community_cards.into());
        if self.total != 0 {
            Some(Equity::from_total_wins_ties(self.total, &self.wins, &self.ties))
        } else {
            None
        }
    }

    fn community_cards(&mut self, remainder: usize) {
        if remainder == 0 {
            let known_cards = self.hero_cards | self.community_cards;
            self.hand_ranking_scores[0] = known_cards.top5().to_score();
            self.known_cards = known_cards;
            self.players(self.villain_ranges.len() - 1);
            return;
        }

        let current_community_cards = self.community_cards;
        let mut current_visited_community_cards = self.visited_community_cards;
        while let Some(card) = (!current_visited_community_cards).first() {
            self.community_cards = current_community_cards.with(card);
            current_visited_community_cards.add(card);
            self.visited_community_cards = current_visited_community_cards;
            self.community_cards(remainder - 1);
        }
    }

    fn players(&mut self, remainder: usize) {
        let player_index = self.villain_ranges.len() - remainder - 1;
        let villain = self.villain_ranges[player_index].as_ref();
        let current_known_cards = self.known_cards;
        villain.for_each_hand(|hand| {
            if current_known_cards.has(hand.high()) || current_known_cards.has(hand.low()) {
                return;
            }

            self.hand_ranking_scores[player_index+1] = self.community_cards
                .with(hand.high())
                .with(hand.low())
                .score_fast();
            self.known_cards = current_known_cards.with(hand.high()).with(hand.low());

            if remainder != 0 {
                self.players(remainder - 1);
            } else {
                self.showdown();
            }
        });
    }

    fn showdown(&mut self) {
        self.total += 1;
        showdown(&self.hand_ranking_scores, &mut self.wins, &mut self.ties)
    }
}

fn showdown(
    hand_ranking_scores: &[Score],
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

pub struct Deck {
    cards: [Card; Card::COUNT],
}

impl Deck {
    pub fn new(rng: &mut impl Rng) -> Self {
        let mut deck = [Card::of(Rank::Two, Suite::Diamonds); Card::COUNT];
        for (i, card) in Card::all().enumerate() {
            deck[i] = card;
        }
        deck.shuffle(rng);
        Deck { cards: deck }
    }

    pub fn draw(
        &mut self,
        rng: &mut impl Rng,
        known_cards: &mut Cards,
    ) -> Option<Card> {
        let mut len = self.cards.len();
        while len > 0 {
            let index = rng.gen_range(0..len);
            let card = self.cards[index];

            if !known_cards.has(card) {
                known_cards.add(card);
                return Some(card);
            }

            self.cards.swap(index, len-1);
            len -= 1;
        }
        None
    }
}
