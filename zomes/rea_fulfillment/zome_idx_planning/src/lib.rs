/**
 * Fulfillment query indexes for planning DNA
 *
 * @package Holo-REA
 * @since   2021-08-29
 */
use hdk::prelude::*;
use hdk_records::{
    index_retrieval::IndexingZomeConfig,
    remote_indexes::{
        RemoteEntryLinkRequest,
        RemoteEntryLinkResponse,
        sync_remote_index,
    },
};

use hc_zome_rea_fulfillment_rpc::*;
use hc_zome_rea_fulfillment_lib_origin::generate_query_handler;
use hc_zome_rea_fulfillment_storage_consts::*;
use hc_zome_rea_commitment_storage_consts::{ COMMITMENT_ENTRY_TYPE, COMMITMENT_FULFILLEDBY_LINK_TAG };

entry_defs![Path::entry_def()];

// :TODO: obviate this with zome-specific configs
#[derive(Clone, Serialize, Deserialize, SerializedBytes, PartialEq, Debug)]
pub struct DnaConfigSlice {
    pub fulfillment_index: IndexingZomeConfig,
}

fn read_index_target_zome(conf: DnaConfigSlice) -> Option<String> {
    Some(conf.fulfillment_index.record_storage_zome)
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchInputs {
    pub params: QueryParams,
}

#[hdk_extern]
fn query_fulfillments(SearchInputs { params }: SearchInputs) -> ExternResult<Vec<ResponseData>>
{
    let handler = generate_query_handler(
        read_index_target_zome,
        COMMITMENT_ENTRY_TYPE,
    );

    Ok(handler(&params)?)
}

#[hdk_extern]
fn _internal_reindex_commitments(indexes: RemoteEntryLinkRequest<CommitmentAddress, FulfillmentAddress>) -> ExternResult<RemoteEntryLinkResponse> {
    let RemoteEntryLinkRequest { remote_entry, target_entries, removed_entries } = indexes;

    Ok(sync_remote_index(
        &COMMITMENT_ENTRY_TYPE, &remote_entry,
        &FULFILLMENT_ENTRY_TYPE,
        target_entries.as_slice(),
        removed_entries.as_slice(),
        &COMMITMENT_FULFILLEDBY_LINK_TAG, &FULFILLMENT_FULFILLS_LINK_TAG,
    )?)
}
