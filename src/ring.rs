use std::sync::Arc;

use crate::{cards::Cards, range::RangeTable};

pub struct Ring {
    hero_position: usize,
    hero_cards: Cards,
    community_cards: Cards,
    cards: Vec<Option<Arc<RangeTable>>>, // None indicates folded or hero cards
    stack_sizes: Vec<u32>, // 1/100 big blinds
}

impl Ring {
    pub fn community_cards(&self) -> Cards {
        self.community_cards
    }

    pub fn hero_cards(&self) -> Cards {
        self.hero_cards
    }

    pub fn villain_cards(&self) -> Vec<Arc<RangeTable>> {
        self.cards.iter().cloned().filter_map(|table| table).collect()
    }
}
