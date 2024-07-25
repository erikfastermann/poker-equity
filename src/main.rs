mod card;
mod cards;
mod equity;
mod hand;
mod range;
mod rank;
mod result;
mod ring;
mod suite;

use std::sync::Arc;

use equity::Equity;

use crate::cards::Cards;
use crate::range::RangeTable;
use crate::result::Result;

fn main() -> Result<()> {
    let range = RangeTable::parse(
        "22+,A2s+,K8s+,Q9s+,J9s+,T9s,98s,87s,ATo+,KJo+,QJo",
        // "AA",
    )?;
    println!("{range}");

    let community_cards = Cards::from_str("KsQs3h").unwrap();
    let hero_cards = Cards::from_str("KhTh").unwrap();
    println!("{community_cards}");
    println!("{hero_cards}");
    let villain_ranges = [Arc::new(range.clone()), Arc::new(range.clone())];

    let equities = Equity::simulate(
        community_cards,
        hero_cards,
        &villain_ranges,
        5_000_000,
    ).unwrap();
    // let equities = Equity::calc(community_cards, hero_cards, &villain_ranges).unwrap();

    for equity in equities {
        println!(
            "equity={:.2} win={:.2} tie={:.2}",
            equity.equity_percent() * 100.0,
            equity.win_percent() * 100.0,
            equity.tie_percent() * 100.0,
        );
    }
    Ok(())
}
