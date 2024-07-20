mod card;
mod cards;
mod equity;
mod range;
mod rank;
mod result;
mod ring;
mod suite;

use std::sync::Arc;

use equity::equity;

use crate::card::Card;
use crate::cards::Cards;
use crate::rank::Rank::*;
use crate::range::parse_range;
use crate::result::Result;
use crate::suite::Suite::*;

fn main() -> Result<()> {
    const RANGE: &'static str = "22+ A2s+ K8s+ Q9s+ J9s+ T9s 98s 87s ATo+ KJo+ QJo+";

    let range = parse_range(RANGE)?;
    println!("{range}");

    let community_cards = Cards::from_slice(&[
        Card::of(Ten, Hearts),
        Card::of(King, Spades),
        Card::of(Ace, Diamonds),
        Card::of(Two, Diamonds),
    ]).unwrap();
    let hero_cards = Cards::from_slice(&[
        Card::of(Ten, Clubs),
        Card::of(Jack, Diamonds),
    ]).unwrap();
    println!("{community_cards}");
    println!("{hero_cards}");

    // let villain_ranges = [Arc::new(RangeTable::full())];
    let villain_ranges = [Arc::new(range)];
    let (won, _, count) = equity(community_cards, hero_cards, &villain_ranges);
    println!("{}", f64::from(won) / f64::from(count));

    Ok(())
}
