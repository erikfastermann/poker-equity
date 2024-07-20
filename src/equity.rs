use std::cmp::Ordering;

use crate::{card::Card, cards::Cards, range::RangeTable};

pub type Ratio = (u32, u32, u32);

pub fn equity(
    community_cards: Cards,
    hero_cards: Cards,
    villain_ranges: &[impl AsRef<RangeTable>],
) -> Ratio {
    assert!(hero_cards.count() == 2);
    assert!(community_cards.count() >= 3 && community_cards.count() <= 5);
    let known_cards = community_cards | hero_cards;
    assert!(known_cards.count() == community_cards.count()+hero_cards.count());
    let remaining_community_cards = 5 - community_cards.count();
    equity_recursive(
        community_cards,
        hero_cards,
        villain_ranges,
        remaining_community_cards,
    )
}

fn equity_recursive(
    community_cards: Cards,
    hero_cards: Cards,
    villain_ranges: &[impl AsRef<RangeTable>],
    remaining_community_cards: u8,
) -> Ratio {
    let known_cards = community_cards | hero_cards;
    let mut won = 0;
    let mut chop = 0;
    let mut count = 0;

    if remaining_community_cards != 0 {
        for card in Card::all() {
            if known_cards.has(card) {
                continue;
            }
            let mut next_community_cards = community_cards;
            next_community_cards.add(card);
            let (next_won, next_chop, next_count) = equity_recursive(
                community_cards,
                hero_cards,
                villain_ranges,
                remaining_community_cards - 1,
            );
            won += next_won;
            chop += next_chop;
            count += next_count;
        }
        return (won, chop, count);
    }

    let hero_top5 = known_cards.top5();
    for villain in villain_ranges {
        let villain = villain.as_ref();
        for card_a in Card::all() {
            if known_cards.has(card_a) {
                continue;
            }
            for card_b in Card::all() {
                if known_cards.has(card_b) {
                    continue;
                }
                if card_a == card_b {
                    continue;
                }
                if !villain.contains(card_a, card_b) {
                    continue;
                }
                let villain_top5 = community_cards.with(card_a).with(card_b).top5();
                count += 1;
                match hero_top5.compare(villain_top5) {
                    Ordering::Less => (),
                    Ordering::Equal => chop += 1,
                    Ordering::Greater => won += 1,
                }
            }
        }
    }
    (won, chop, count)
}
