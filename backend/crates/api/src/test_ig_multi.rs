use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct IgMultiMarketsResponse {
    pub marketDetails: Vec<IgMarketDetail>,
}

#[derive(Deserialize, Debug)]
pub struct IgMarketDetail {
    pub instrument: IgInstrument,
    pub snapshot: IgSnapshotPrix,
}

#[derive(Deserialize, Debug)]
pub struct IgInstrument {
    pub epic: String,
}

#[derive(Deserialize, Debug)]
pub struct IgSnapshotPrix {
    pub bid: Option<f64>,
    pub offer: Option<f64>,
}
