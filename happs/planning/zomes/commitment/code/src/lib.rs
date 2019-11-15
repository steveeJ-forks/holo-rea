/**
 * Planning zome API definition
 *
 * # Remarks
 *
 * Defines the top-level zome configuration needed by Holochain's build system
 * to bundle the app. This basically involves wiring up the `entry!` type macros
 * and `define_zome!` definition to the standard Rust code in the rest of this
 * module.
 *
 * @package: HoloREA
 * @author:  pospi <pospi@spadgos.com>
 * @since:   2019-02-06
 */

extern crate hdk;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate hdk_graph_helpers;
extern crate vf_planning;

mod commitment_requests;

use hdk::prelude::*;

use vf_planning::type_aliases::CommitmentAddress;
use vf_planning::commitment::{
    Entry as CommitmentEntry,
    CreateRequest as CommitmentCreateRequest,
    UpdateRequest as CommitmentUpdateRequest,
    ResponseData as CommitmentResponse,
};

use commitment_requests::{
    QueryParams,
    receive_get_commitment,
    receive_create_commitment,
    receive_update_commitment,
    receive_delete_commitment,
    receive_query_commitments,
};

use vf_planning::identifiers::{
    COMMITMENT_BASE_ENTRY_TYPE,
    COMMITMENT_INITIAL_ENTRY_LINK_TYPE,
    COMMITMENT_ENTRY_TYPE,
    COMMITMENT_FULFILLEDBY_LINK_TYPE,
    FULFILLMENT_BASE_ENTRY_TYPE,
    COMMITMENT_SATISFIES_LINK_TYPE,
    SATISFACTION_BASE_ENTRY_TYPE,
    COMMITMENT_INPUT_OF_LINK_TYPE,
    COMMITMENT_OUTPUT_OF_LINK_TYPE,
};
use vf_observation::identifiers::{
    PROCESS_BASE_ENTRY_TYPE,
    PROCESS_COMMITMENT_INPUTS_LINK_TYPE, PROCESS_COMMITMENT_OUTPUTS_LINK_TYPE,
    PROCESS_INTENT_INPUTS_LINK_TYPE, PROCESS_INTENT_OUTPUTS_LINK_TYPE,
};
use vf_planning::identifiers::{
    INTENT_BASE_ENTRY_TYPE,
};

// Zome entry type wrappers

fn commitment_entry_def() -> ValidatingEntryType {
    entry!(
        name: COMMITMENT_ENTRY_TYPE,
        description: "A planned economic flow that has been promised by an agent to another agent.",
        sharing: Sharing::Public,
        validation_package: || {
            hdk::ValidationPackageDefinition::Entry
        },
        validation: |_validation_data: hdk::EntryValidationData<CommitmentEntry>| {
            // CREATE
            if let EntryValidationData::Create{ entry, validation_data: _ } = validation_data {
                let record: CommitmentEntry = entry;
                if !(record.resource_inventoried_as.is_some() || record.resource_classified_as.is_some() || record.resource_conforms_to.is_some()) {
                    return Err("Commitment must reference an inventoried resource, resource specification or resource classification".into());
                }
                if !(record.resource_quantity.is_some() || record.effort_quantity.is_some()) {
                    return Err("Commmitment must include either a resource quantity or an effort quantity".into());
                }
                if !(record.has_beginning.is_some() || record.has_end.is_some() || record.has_point_in_time.is_some() || record.due.is_some()) {
                    return Err("Commmitment must have a beginning, end, exact time or due date".into());
                }
            }

            // UPDATE
            // if let EntryValidationData::Modify{ new_entry, old_entry, old_entry_header: _, validation_data: _ } = validation_data {

            // }

            // DELETE
            // if let EntryValidationData::Delete{ old_entry, old_entry_header: _, validation_data: _ } = validation_data {

            // }

            Ok(())
        }
    )
}

fn commitment_base_entry_def() -> ValidatingEntryType {
    entry!(
        name: COMMITMENT_BASE_ENTRY_TYPE,
        description: "Base anchor for initial commitment addresses to provide lookup functionality",
        sharing: Sharing::Public,
        validation_package: || {
            hdk::ValidationPackageDefinition::Entry
        },
        validation: |_validation_data: hdk::EntryValidationData<Address>| {
            Ok(())
        },
        links: [
            to!(
                COMMITMENT_ENTRY_TYPE,
                link_type: COMMITMENT_INITIAL_ENTRY_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            ),
            to!(
                FULFILLMENT_BASE_ENTRY_TYPE,
                link_type: COMMITMENT_FULFILLEDBY_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            ),
            to!(
                SATISFACTION_BASE_ENTRY_TYPE,
                link_type: COMMITMENT_SATISFIES_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            ),
            to!(
                PROCESS_BASE_ENTRY_TYPE,
                link_type: COMMITMENT_INPUT_OF_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            ),
            to!(
                PROCESS_BASE_ENTRY_TYPE,
                link_type: COMMITMENT_OUTPUT_OF_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            )
        ]
    )
}

fn process_base_entry_def() -> ValidatingEntryType {
    entry!(
        name: PROCESS_BASE_ENTRY_TYPE,
        description: "Base anchor for processes being linked to in external networks",
        sharing: Sharing::Public,
        validation_package: || {
            hdk::ValidationPackageDefinition::Entry
        },
        validation: |_validation_data: hdk::EntryValidationData<Address>| {
            Ok(())
        },
        links: [
            to!(
                COMMITMENT_BASE_ENTRY_TYPE,
                link_type: PROCESS_COMMITMENT_INPUTS_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            ),
            to!(
                COMMITMENT_BASE_ENTRY_TYPE,
                link_type: PROCESS_COMMITMENT_OUTPUTS_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            ),
            // :TODO: ideally this would be defined on a separate `PROCESS_BASE_ENTRY_TYPE`
            // in the intent zome.
            // This might point to a need to split `Process` functionality out into its own zome
            // within the planning DNA.
            to!(
                INTENT_BASE_ENTRY_TYPE,
                link_type: PROCESS_INTENT_INPUTS_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            ),
            to!(
                INTENT_BASE_ENTRY_TYPE,
                link_type: PROCESS_INTENT_OUTPUTS_LINK_TYPE,
                validation_package: || {
                    hdk::ValidationPackageDefinition::Entry
                },
                validation: | _validation_data: hdk::LinkValidationData| {
                    Ok(())
                }
            )
        ]
    )
}

// Zome definition

define_zome! {
    entries: [
        commitment_entry_def(),
        commitment_base_entry_def(),
        process_base_entry_def()
    ]

    init: || {
        Ok(())
    }

    validate_agent: |validation_data : EntryValidationData::<AgentId>| {
        Ok(())
    }

    receive: |from, payload| {
      format!("Received: {} from {}", payload, from)
    }

    functions: [
        create_commitment: {
            inputs: |commitment: CommitmentCreateRequest|,
            outputs: |result: ZomeApiResult<CommitmentResponse>|,
            handler: receive_create_commitment
        }
        get_commitment: {
            inputs: |address: CommitmentAddress|,
            outputs: |result: ZomeApiResult<CommitmentResponse>|,
            handler: receive_get_commitment
        }
        update_commitment: {
            inputs: |commitment: CommitmentUpdateRequest|,
            outputs: |result: ZomeApiResult<CommitmentResponse>|,
            handler: receive_update_commitment
        }
        delete_commitment: {
            inputs: |address: CommitmentAddress|,
            outputs: |result: ZomeApiResult<bool>|,
            handler: receive_delete_commitment
        }

        query_commitments: {
            inputs: |params: QueryParams|,
            outputs: |result: ZomeApiResult<Vec<CommitmentResponse>>|,
            handler: receive_query_commitments
        }
    ]

    traits: {
        hc_public [
            create_commitment,
            get_commitment,
            update_commitment,
            delete_commitment,
            query_commitments
        ]
    }
}
