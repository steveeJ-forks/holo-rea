/**
 * Helpers related to `local indexes`.
 *
 * A `local index` is a simple set of links between Holochain entries. These are
 * appropriate for linking directly between entries within the same DNA.
 *
 * @see     ../README.md
 * @package HDK Graph Helpers
 * @since   2019-05-16
 */
use hdk3::prelude::*;

use crate::{
    GraphAPIResult,
    record_interface::Identified,
    internals::*,
    identity_helpers::{
        calculate_identity_address,
        read_entry_identity,
    },
    links::{
        get_linked_headers,
        get_linked_addresses,
    },
    records::{
        get_records_by_identity_address,
    },
};

//--------------------------------[ READ ]--------------------------------------

/// Reads and returns all entry identities referenced by the given index from
/// (`base_entry_type.base_address` via `link_tag`.
///
/// Use this method to query associated IDs for a query edge, without retrieving
/// the records themselves.
///
pub fn read_index<'a, A, S: 'a + AsRef<[u8]>, I: AsRef<str>>(
    base_entry_type: &I,
    base_address: &EntryHash,
    link_tag: S,
) -> GraphAPIResult<Vec<GraphAPIResult<A>>>
    where A: From<EntryHash>,
{
    Ok(read_index_entry_hashes(base_entry_type, base_address, link_tag)?.iter()
        .map(|i| { Ok(A::from((*i).clone()?)) })
        .collect())
}

/// Internal version of `read_index` which returns unwrapped `EntryHash` types; for internal library use only.
///
fn read_index_entry_hashes<'a, S: 'a + AsRef<[u8]>, I: AsRef<str>>(
    base_entry_type: &I,
    base_address: &EntryHash,
    link_tag: S,
) -> GraphAPIResult<Vec<GraphAPIResult<EntryHash>>>
{
    let index_address = calculate_identity_address(base_entry_type, base_address)?;
    let refd_index_addresses = get_linked_addresses(&index_address, LinkTag::new(link_tag.as_ref()))?;

    Ok(refd_index_addresses.iter()
        .map(read_entry_identity)
        .collect())
}

/// Given a base address to query from, returns a Vec of tuples of all target
/// `EntryHash`es referenced via the given link tag, bound to the result of
/// attempting to decode each referenced entry into the requested type `R`.
///
/// Use this method to query associated records for a query edge in full.
///
pub fn query_index<'a, T, R, A, S: 'a + AsRef<[u8]>, I: AsRef<str>>(
    base_entry_type: &I,
    base_address: &EntryHash,
    link_tag: S,
) -> GraphAPIResult<Vec<GraphAPIResult<(HeaderHash, A, T)>>>
    where A: From<EntryHash>,
        SerializedBytes: TryInto<R, Error = SerializedBytesError>,
        R: Identified<T>,
{
    let index_address = calculate_identity_address(base_entry_type, base_address)?;
    let addrs_result = get_linked_addresses(&index_address, LinkTag::new(link_tag.as_ref()))?;
    let entries = get_records_by_identity_address::<T, R, A>(&addrs_result);
    Ok(entries)
}

//-------------------------------[ CREATE ]-------------------------------------

/// Creates a bidirectional link between two entry addresses, and returns a vector
/// of the `HeaderHash`es of the (respectively) forward & reciprocal links created.
pub fn create_index<'a, S: 'a + AsRef<[u8]>, I: AsRef<str>>(
    source_entry_type: &I,
    source: &EntryHash,
    dest_entry_type: &I,
    dest: &EntryHash,
    link_tag: S,
    link_tag_reciprocal: S,
) -> GraphAPIResult<Vec<HeaderHash>> {
    let source_hash = calculate_identity_address(source_entry_type, source)?;
    let dest_hash = calculate_identity_address(dest_entry_type, dest)?;

    Ok(vec! [
        create_link(source_hash.clone(), dest_hash.clone(), LinkTag::new(link_tag.as_ref()))?,
        create_link(dest_hash, source_hash, LinkTag::new(link_tag_reciprocal.as_ref()))?,
    ])
}

//-------------------------------[ UPDATE ]-------------------------------------

/// Updates an index set from some originating entry located at the Path `source_entry_type`.`source`.
///
/// The destination entry is stored at the Path prefix `dest_entry_type`. For this prefix, any entry
/// identifiers in `add_dest_addresses` which are not already linked will have indexes created.
///
/// Any indexes which are already present between the source and addresses in `remove_dest_addresses`
/// will be removed.
///
/// An update for a single entry is thus performed by specifiying the previous entry ID in
/// `remove_dest_addresses`, and the new entry ID in `add_dest_addresses`.
///
pub fn update_index<'a, S: 'a + AsRef<[u8]>, I: AsRef<str>>(
    source_entry_type: &I,
    source: &EntryHash,
    dest_entry_type: &I,
    link_tag: &S,
    link_tag_reciprocal: &S,
    add_dest_addresses: &[EntryHash],
    remove_dest_addresses: &[EntryHash],
) -> GraphAPIResult<Vec<GraphAPIResult<HeaderHash>>>
{
    // load any existing linked entries from the originating address
    let (existing_link_results, read_errors): (Vec<GraphAPIResult<EntryHash>>, Vec<GraphAPIResult<EntryHash>>) = read_index_entry_hashes(source_entry_type, source, link_tag)?
        .iter().cloned()
        .partition(result_partitioner);

    // eagerly throw errors (:TODO: explore safer / 'more eager to update' ways of handling such data corruption)
    throw_any_error(read_errors)?;

    let existing_links: Vec<EntryHash> = existing_link_results
        .iter().cloned()
        .map(Result::unwrap)
        .collect();

    // determine links to erase
    let to_erase: Vec<EntryHash> = existing_links
        .iter()
        .filter(link_matches(remove_dest_addresses))
        .cloned()
        .collect();

    // wipe any indexes flagged for removal
    let delete_index_results: Vec<GraphAPIResult<HeaderHash>> = to_erase
        .iter()
        .flat_map(delete_dest_indexes(source_entry_type, source, dest_entry_type, link_tag, link_tag_reciprocal))
        .collect();

    // check which inserts are needed
    let already_present: Vec<EntryHash> = existing_links
        .iter()
        .filter(link_matches(add_dest_addresses))
        .cloned()
        .collect();

    let to_add = vect_difference(&existing_links, &already_present);

    // add any new links not already present
    let create_index_results: Vec<GraphAPIResult<HeaderHash>> = to_add
        .iter()
        .flat_map(create_dest_indexes(source_entry_type, source, dest_entry_type, link_tag, link_tag_reciprocal))
        .collect();

    Ok(delete_index_results
        .iter().cloned().chain(
            create_index_results.iter().cloned()
        ).collect()
    )
}

//-------------------------------[ DELETE ]-------------------------------------

/// Deletes a bidirectional link between two entry addresses. Any active links between
/// the given addresses using the given tags will be deleted.
///
pub fn delete_index<'a, S: 'a + AsRef<[u8]>, I: AsRef<str>>(
    source_entry_type: &I,
    source: &EntryHash,
    dest_entry_type: &I,
    dest: &EntryHash,
    link_tag: S,
    link_tag_reciprocal: S,
) -> GraphAPIResult<Vec<GraphAPIResult<HeaderHash>>> {
    let tag_source = LinkTag::new(link_tag.as_ref());
    let tag_dest = LinkTag::new(link_tag_reciprocal.as_ref());
    let address_source = calculate_identity_address(source_entry_type, source)?;
    let address_dest = calculate_identity_address(dest_entry_type, dest)?;

    let mut links = get_linked_headers(&address_source, tag_source)?;
    links.append(& mut get_linked_headers(&address_dest, tag_dest)?);

    Ok(links
        .iter()
        .map(|l| { Ok(delete_link((*l).clone())?) })
        .collect()
    )
}
