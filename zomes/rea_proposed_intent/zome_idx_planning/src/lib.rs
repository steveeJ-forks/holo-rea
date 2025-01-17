#![feature(proc_macro_hygiene)]
/**
* Holo-REA proposed intents zome API definition
*
* Defines the top-level zome configuration needed by Holochain's build system
* to bundle the app. This basically involves wiring up the helper methods from the
* related `_lib` module into a packaged zome WASM binary.
*
* @package Holo-REA
*/
extern crate hdk;
extern crate hdk_proc_macros;
extern crate serde;

use hdk::prelude::*;
use hdk_proc_macros::zome;

// use hdk_records::remote_indexes::RemoteEntryLinkRespnse; // :TODO: wire up remote indexing API if necessary

use hc_zome_rea_proposed_intent_defs::{base_entry_def, entry_def};
use hc_zome_rea_proposed_intent_lib_destination_planning::*;
use hc_zome_rea_proposed_intent_rpc::*;

// Zome entry type wrappers
#[zome]
mod rea_proposed_intent_zome {

    #[init]
    fn init() {
        Ok(())
    }

    #[validate_agent]
    pub fn validate_agent(validation_data: EntryValidationData<AgentId>) {
        Ok(())
    }

    #[entry_def]
    fn proposed_intent_entry_def() -> ValidatingEntryType {
        entry_def()
    }

    #[entry_def]
    fn proposed_intent_base_entry_def() -> ValidatingEntryType {
        base_entry_def()
    }

    #[zome_fn("hc_public")]
    fn created_proposed_intent(proposed_intent: CreateRequest) -> ZomeApiResult<ResponseData> {
        handle_create_proposed_intent(proposed_intent)
    }

    #[zome_fn("hc_public")]
    fn get_proposed_intent(address: ProposedIntentAddress) -> ZomeApiResult<ResponseData> {
        handle_get_proposed_intent(address)
    }

    #[zome_fn("hc_public")]
    fn deleted_proposed_intent(address: ProposedIntentAddress) -> ZomeApiResult<bool> {
        handle_delete_proposed_intent(address)
    }

    // :TODO:
    // receive: |from, payload| {
    //     format!("Received: {} from {}", payload, from)
    // }
}
