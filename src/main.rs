mod card;
mod cards;
mod equity;
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
    const RANGE: &'static str = "22+,A2s+,K8s+,Q9s+,J9s+,T9s,98s,87s,ATo+,KJo+,QJo";

    let range = RangeTable::parse(RANGE)?;
    println!("{range}");

    let community_cards = Cards::from_str("ThQsAd").unwrap();
    let hero_cards = Cards::from_str("KsTd").unwrap();
    println!("{community_cards}");
    println!("{hero_cards}");

    // let villain_ranges = [Arc::new(RangeTable::full())];
    let villain_ranges = [Arc::new(range.clone()), Arc::new(range)];
    let equity = Equity::equity(community_cards, hero_cards, &villain_ranges);
    println!(
        "equity={} win={} tie={} ({:?})",
        equity.equity_percent(),
        equity.win_percent(),
        equity.tie_percent(),
        equity,
    );

    Ok(())
}
