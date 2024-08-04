use std::cmp::min;

use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};

use crate::{card::Card, cards::{Cards, Score}, hand::Hand, range::RangeTable};

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
    assert!(villain_ranges.iter().all(|range| !range.as_ref().is_empty()));
}

pub fn total_combos_upper_bound(
    community_cards: Cards,
    villain_ranges: &[impl AsRef<RangeTable>],
) -> u128 {
    assert!(villain_ranges.len() <= 8);
    assert!(villain_ranges.iter().all(|range| !range.as_ref().is_empty()));
    let community_cards_count = community_cards.count();
    assert!(community_cards_count <= 5);
    let mut remaining_cards = {
        let remaining_cards = Card::COUNT - usize::from(community_cards_count) - 2;
        u128::try_from(remaining_cards).unwrap()
    };
    let mut count = 1u128;

    for _ in community_cards_count..5 {
        count *= remaining_cards;
        remaining_cards -= 1;
    }

    let mut max_count = count;
    for _ in 0..villain_ranges.len()*2 {
        max_count *= remaining_cards;
        remaining_cards -= 1;
    }

    for range in villain_ranges {
        let next_count = count.checked_mul(u128::from(range.as_ref().count_cards()));
        match next_count {
            Some(n) => count = n,
            None => return u128::MAX,
        };
    }

    min(count, max_count)
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
        check_input(start_community_cards, hero_cards, villain_ranges);
        let mut rng = SmallRng::from_entropy();
        let remaining_community_cards = 5 - start_community_cards.count();
        let player_count = villain_ranges.len() + 1;

        let mut total = 0u64;
        let mut scores = vec![Score::ZERO; player_count];
        let mut wins = vec![0u64; player_count];
        let mut ties = vec![0.0; player_count];

        let mut deck = Deck::from_cards(&mut rng, start_community_cards | hero_cards);

        'outer: for _ in 0..rounds {
            deck.reset();

            let community_cards = {
                let mut community_cards = start_community_cards;
                for _ in 0..remaining_community_cards {
                    community_cards.add(deck.draw(&mut rng).unwrap());
                }
                community_cards
            };

            scores[0] = (community_cards | hero_cards).score_fast();
            for i in 1..player_count {
                let hand = deck.hand(&mut rng).unwrap();
                if !villain_ranges[i-1].as_ref().contains(hand) {
                    continue 'outer;
                }
                let player_cards = community_cards.with(hand.high()).with(hand.low());
                scores[i] = player_cards.score_fast();
            }

            total += 1;
            showdown(&scores, &mut wins, &mut ties);
        }

        if total == 0 {
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

    fn calc(mut self) -> Option<Vec<Equity>> {
        let upper_bound = total_combos_upper_bound(
            self.community_cards,
            self.villain_ranges,
        );
        if u64::try_from(upper_bound).is_err() {
            return None;
        }
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
    max_len: usize,
    len: usize,
}

impl Deck {
    pub fn from_cards(rng: &mut impl Rng, known_cards: Cards) -> Self {
        let mut cards = [Card::MIN; Card::COUNT];
        let mut index = 0;
        for card in Card::all() {
            if known_cards.has(card) {
                continue;
            }
            cards[index] = card;
            index += 1;
        }
        cards[..index].shuffle(rng);
        Deck { cards, max_len: index, len: index }
    }

    pub fn draw(&mut self, rng: &mut impl Rng) -> Option<Card> {
        if self.len == 0 {
            None
        } else {
            let index = rng.gen_range(0..self.len);
            let card = self.cards[index];
            self.cards.swap(index, self.len-1);
            self.len -= 1;
            Some(card)
        }
    }

    pub fn hand(&mut self, rng: &mut impl Rng) -> Option<Hand> {
        let a = self.draw(rng)?;
        let b = self.draw(rng)?;
        Some(Hand::of_cards(a, b))
    }

    pub fn reset(&mut self) {
        self.len = self.max_len;
    }
}

struct CardGenerator<'a, RT: AsRef<RangeTable>> {
    known_cards: Cards,
    hero_cards: Cards,
    visited_community_cards: Cards,
    community_cards: Cards,
    villain_ranges: &'a [RT],
    cards: Vec<(Cards, [Hand; 9])>,
    current_hands: [Hand; 9],
}

impl <'a, RT: AsRef<RangeTable>> CardGenerator<'a, RT> {
    fn new(
        community_cards: Cards,
        hero_cards: Cards,
        villain_ranges: &'a [RT],
    ) -> Self {
        check_input(community_cards, hero_cards, villain_ranges);
        let mut calculator = Self {
            known_cards: Cards::EMPTY,
            hero_cards,
            community_cards,
            visited_community_cards: community_cards | hero_cards,
            villain_ranges,
            cards: Vec::new(),
            current_hands: [Hand::MIN; 9],
        };
        calculator.current_hands[0] = hero_cards.to_hand().unwrap();
        calculator
    }

    fn build(mut self) -> Vec<(Cards, [Hand; 9])> {
        let remaining_community_cards = 5 - self.community_cards.count();
        self.community_cards(remaining_community_cards.into());
        self.cards
    }

    fn community_cards(&mut self, remainder: usize) {
        if remainder == 0 {
            let known_cards = self.hero_cards | self.community_cards;
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

            let hand_index = 1 + (self.villain_ranges.len() - remainder - 1);
            self.current_hands[hand_index] = hand;
            self.known_cards = current_known_cards.with(hand.high()).with(hand.low());

            if remainder != 0 {
                self.players(remainder - 1);
            } else {
                self.cards.push((self.community_cards, self.current_hands));
            }
        });
    }
}

struct CardDistribution<'a, RT: AsRef<RangeTable>> {
    known_cards: Cards,
    hero_cards: Cards,
    visited_community_cards: Cards,
    community_cards: Cards,
    villain_ranges: &'a [RT],
    community_distribution: Vec<u64>,
    current_hands: [Hand; 9],
    hand_distributions: Vec<Vec<u64>>,
}

impl <'a, RT: AsRef<RangeTable>> CardDistribution<'a, RT> {
    fn new(
        community_cards: Cards,
        hero_cards: Cards,
        villain_ranges: &'a [RT],
    ) -> Self {
        check_input(community_cards, hero_cards, villain_ranges);
        let player_count = villain_ranges.len() + 1;
        let mut distribution = Self {
            known_cards: Cards::EMPTY,
            hero_cards,
            community_cards,
            visited_community_cards: community_cards | hero_cards,
            villain_ranges,
            community_distribution: vec![0; 64],
            current_hands: [Hand::MIN; 9],
            hand_distributions: vec![vec![0; 64*64]; player_count],
        };
        let hero_hand_index = hero_cards.to_hand().unwrap().to_index();
        distribution.hand_distributions[0][hero_hand_index] = 1;
        distribution
    }

    fn calc(mut self) -> (Vec<u64>, Vec<Vec<u64>>) {
        let remaining_community_cards = 5 - self.community_cards.count();
        self.community_cards(remaining_community_cards.into());
        (self.community_distribution, self.hand_distributions)
    }

    fn community_cards(&mut self, remainder: usize) {
        if remainder == 0 {
            self.known_cards = self.hero_cards | self.community_cards;
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

            let player_index = 1 + (self.villain_ranges.len() - remainder - 1);
            self.current_hands[player_index] = hand;
            self.known_cards = current_known_cards.with(hand.high()).with(hand.low());

            if remainder != 0 {
                self.players(remainder - 1);
            } else {
                for card in self.community_cards.iter() {
                    self.community_distribution[card.to_usize()] += 1;
                }
                let hands = &self.current_hands[..self.hand_distributions.len()];
                for (index, hand) in hands.iter().copied().enumerate() {
                    self.hand_distributions[index][hand.to_index()] += 1;
                }
            }
        });
    }
}
