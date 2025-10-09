use mosaic_fi::note::{Market, MosaicNote};

pub async fn post_note(_market: Market, _note: MosaicNote) -> Result<(), ()> {
    Ok(())
}

pub async fn get_notes(_market: Market) -> Result<Vec<MosaicNote>, ()> {
    Ok(vec![])
}
