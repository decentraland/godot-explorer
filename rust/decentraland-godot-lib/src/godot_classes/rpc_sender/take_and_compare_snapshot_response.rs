use godot::prelude::*;

use crate::{
    dcl::js::testing::{GreyPixelDiffResult, TakeAndCompareSnapshotResponse},
    generate_dcl_rpc_sender,
};

impl TakeAndCompareSnapshotResponse {
    fn from_variant(via: Variant) -> Option<Self> {
        let via = via.to::<Dictionary>();
        let stored_snapshot_found = via.get("stored_snapshot_found")?.to::<bool>();
        let grey_pixel_diff = via.get("grey_pixel_diff").map(|grey| GreyPixelDiffResult {
            similarity: grey
                .to::<Dictionary>()
                .get("similarity")
                .expect("similarity")
                .to::<f64>(),
        });

        Some(Self {
            stored_snapshot_found,
            grey_pixel_diff,
        })
    }
}

generate_dcl_rpc_sender!(
    DclRpcSenderTakeAndCompareSnapshotResponse,
    TakeAndCompareSnapshotResponse
);
