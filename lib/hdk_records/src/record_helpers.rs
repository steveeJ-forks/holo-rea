/**
 * Record-handling abstractions for Holochain apps
 *
 * Allows for data layers which behave like a traditional graph database, where
 * 'records' are the core abstraction, managed by complex arrangements of DHT
 * entries and links.
 *
 * @package HoloREA
 * @since   2019-07-02
 */
use hdk::prelude::*;
use hdk::info::zome_info;

use crate::{
    RevisionHash, DnaAddressable,
    RecordAPIResult, DataIntegrityError,
    record_interface::{Identifiable, Identified, Updateable},
    entries::{
        get_entry_by_header,
        create_entry,
        update_entry,
        delete_entry,
    },
    identity_helpers::{
        create_entry_identity,
        read_entry_identity,
        calculate_identity_address,
    },
};

/// Helper to retrieve the HeaderHash for an Element
///
fn get_header_hash(shh: element::SignedHeaderHashed) -> HeaderHash {
    shh.header_hashed().as_hash().to_owned()
}

//--------------------------------[ READ ]--------------------------------------

/// Retrieve the latest available RevisionHash for a given EntryHash.
///
/// Useful in coordinating updates between different entry types.
///
pub fn get_latest_header_hash(entry_hash: EntryHash) -> RecordAPIResult<RevisionHash> {
    Ok(RevisionHash(zome_info()?.dna_hash, (match get_details(entry_hash, GetOptions { strategy: GetStrategy::Latest })? {
        Some(Details::Entry(details)) => match details.entry_dht_status {
            metadata::EntryDhtStatus::Live => match details.updates.len() {
                0 => {
                    // no updates yet, latest header hash is the first one
                    Ok(get_header_hash(details.headers.first().unwrap().to_owned()))
                },
                _ => {
                    // updates exist, find most recent header
                    let mut sortlist = details.updates.to_vec();
                    sortlist.sort_by_key(|update| update.header().timestamp().as_micros());
                    let last = sortlist.last().unwrap().to_owned();
                    Ok(get_header_hash(last))
                },
            },
            _ => Err(DataIntegrityError::EntryNotFound),
        },
        _ => Err(DataIntegrityError::EntryNotFound),
    })?))
}

/// Retrive the specific version of an entry specified by the given `RevisionHash`
///
pub fn read_record_entry_by_header<T, R, B>(
    header_hash: &RevisionHash,
) -> RecordAPIResult<(B, T)>
    where T: std::fmt::Debug,
        B: DnaAddressable<EntryHash>,
        SerializedBytes: TryInto<R, Error = SerializedBytesError>,
        Entry: TryFrom<R>,
        R: std::fmt::Debug + Identified<T, B>,
{
    let storage_entry: R = get_entry_by_header(&header_hash)?;
    Ok((storage_entry.identity()?, storage_entry.entry()))
}

/// Read a record's entry data by its identity index
///
/// :TODO: Currently, the most recent version of the given entry will
///        be provided instead of the exact entry specified.
///        We should also check for multiple live headers, and throw a
///        conflict error if necessary. But core may implement this for
///        us eventually. (@see EntryDhtStatus)
///
pub (crate) fn read_record_entry_by_identity<T, R, B>(
    identity_address: &EntryHash,
) -> RecordAPIResult<(RevisionHash, B, T)>
    where T: std::fmt::Debug,
        B: DnaAddressable<EntryHash>,
        SerializedBytes: TryInto<R, Error = SerializedBytesError>,
        Entry: TryFrom<R>,
        R: std::fmt::Debug + Identified<T, B>,
{
    // read active links to current version
    let entry_hash = read_entry_identity(identity_address)?;
    // pull details of the current version, to ensure we have the most recent
    let latest_header_hash = get_latest_header_hash(entry_hash)?;

    let (entry_hash, entry_data) = read_record_entry_by_header(&latest_header_hash)?;

    Ok((latest_header_hash, entry_hash, entry_data))
}

/// Read a record's entry data by locating it via an anchor `Path` composed
/// of some root component and (uniquely identifying) initial identity address.
///
/// Presumes that the record is to be fetched from the current DNA and naturally errors
/// if attempted on an `EntryHash` that only exists in a foreign cell.
///
pub fn read_record_entry<T, R, B, S>(
    entry_type_root_path: &S,
    address: &EntryHash,
) -> RecordAPIResult<(RevisionHash, B, T)>
    where S: AsRef<str>,
        T: std::fmt::Debug,
        B: DnaAddressable<EntryHash>,
        SerializedBytes: TryInto<R, Error = SerializedBytesError>,
        Entry: TryFrom<R>,
        R: std::fmt::Debug + Identified<T, B>,
{
    let identity_address = calculate_identity_address(entry_type_root_path, &B::new(zome_info()?.dna_hash, address.clone()))?;
    read_record_entry_by_identity::<T, R, B>(&identity_address)
}

//-------------------------------[ CREATE ]-------------------------------------

/// Creates a new record in the DHT, assigns it an identity index (@see identity_helpers.rs)
/// and returns a tuple of this version's `HeaderHash`, the identity `EntryHash` and initial record `entry` data.
///
pub fn create_record<I, R: Clone, B, C, E, S>(
    entry_def_id: S,
    create_payload: C,
) -> RecordAPIResult<(RevisionHash, B, I)>
    where S: AsRef<str>,
        B: DnaAddressable<EntryHash>,
        C: Into<I>,
        I: Identifiable<R>,
        WasmError: From<E>,
        Entry: TryFrom<R, Error = E>,
        R: Identified<I, B>,
{
    // convert the type's CREATE payload into internal storage struct
    let entry_data: I = create_payload.into();
    // wrap data with null identity for origin record
    let storage = entry_data.with_identity(None);

    // write underlying entry
    let (header_hash, entry_hash) = create_entry(&entry_def_id, storage)?;

    // create an identifier for the new entry
    let identity = B::new(zome_info()?.dna_hash, entry_hash.clone());
    let identity_address = create_entry_identity(&entry_def_id, &identity)?;

    // link the identifier to the actual entry
    create_link(identity_address, entry_hash, LinkTag::new(crate::identifiers::RECORD_INITIAL_ENTRY_LINK_TAG))?;

    Ok((header_hash, identity, entry_data))
}

//-------------------------------[ UPDATE ]-------------------------------------

/// Updates a record in the DHT by its `HeaderHash` (revision ID)
///
/// The way in which the input update payload is applied to the existing
/// entry data is up to the implementor of `Updateable<U>` for the entry type.
///
/// :TODO: prevent multiple updates to the same HeaderHash under standard operations
///
/// @see hdk_records::record_interface::Updateable
///
pub fn update_record<I, R: Clone, B, U, E, S>(
    entry_def_id: S,
    address: &RevisionHash,
    update_payload: U,
) -> RecordAPIResult<(RevisionHash, B, I, I)>
    where S: AsRef<str>,
        B: DnaAddressable<EntryHash>,
        I: Identifiable<R> + Updateable<U>,
        WasmError: From<E>,
        Entry: TryFrom<R, Error = E>,
        R: Clone + Identified<I, B>,
        SerializedBytes: TryInto<R, Error = SerializedBytesError>,
{
    // get referenced entry for the given header
    let previous: R = get_entry_by_header(address)?;
    let prev_entry = previous.entry();
    let identity = previous.identity()?;
    let identity_hash: &EntryHash = identity.as_ref();

    // apply update payload
    let new_entry = prev_entry.update_with(update_payload);
    let storage: R = new_entry.with_identity(Some(identity_hash.clone()));

    // perform regular entry update using internal address
    let (header_addr, _entry_addr) = update_entry(&entry_def_id, address, storage)?;

    Ok((header_addr, identity, new_entry, prev_entry))
}

//-------------------------------[ DELETE ]-------------------------------------

/// Removes a record of the given `HeaderHash` from the DHT by marking it as deleted.
///
/// Links are not affected so as to retain a link to the referencing information, which may now need to be updated.
///
pub fn delete_record<T, A>(address: &A) -> RecordAPIResult<bool>
    where SerializedBytes: TryInto<T, Error = SerializedBytesError>,
        A: AsRef<HeaderHash>,
{
    // :TODO: handle deletion of the identity `Path` for the referenced entry if this is the last header being deleted

    delete_entry::<T, A>(address)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdk_type_serialization_macros::{ simple_alias, addressable_identifier };
    use crate::{generate_record_entry};

    simple_alias!(EntryId => EntryHash);

    #[derive(Clone, Serialize, Deserialize, SerializedBytes, PartialEq, Debug)]
    pub struct Entry {
        field: Option<String>,
    }
    generate_record_entry!(Entry, EntryWithIdentity);

    #[derive(Clone)]
    pub struct CreateRequest {
        field: Option<String>,
    }

    impl From<CreateRequest> for Entry {
        fn from(e: CreateRequest) -> Entry {
            Entry {
                field: e.field.into(),
            }
        }
    }

    #[derive(Clone)]
    pub struct UpdateRequest {
        field: Option<String>,
    }

    impl Updateable<UpdateRequest> for Entry {
        fn update_with(&self, e: UpdateRequest) -> Entry {
            Entry {
                field: e.field.to_owned(),
            }
        }
    }

    #[test]
    fn test_roundtrip() {
        let entry_type: String = "testing".to_string();

        // CREATE
        let (header_addr, base_address, initial_entry): (_, EntryId, Entry) = create_record(&entry_type, CreateRequest { field: None }).unwrap();

        // Verify read
        let (header_addr_2, returned_address, first_entry) = read_record_entry::<Entry, EntryWithIdentity, EntryId,_,_>(&entry_type, &base_address).unwrap();
        assert_eq!(header_addr, header_addr_2, "record should have same header ID on read as for creation");
        assert_eq!(base_address.as_ref(), returned_address.as_ref(), "record should have same identifier ID on read as for creation");
        assert_eq!(initial_entry, first_entry, "record from creation output should be same as read data");

        // UPDATE
        let (updated_header_addr, identity_address, updated_entry): (_, EntryId, Entry) = update_record(&entry_type, &header_addr, UpdateRequest { field: Some("value".into()) }).unwrap();

        // Verify update & read
        assert_eq!(base_address.as_ref(), identity_address.as_ref(), "record should have consistent ID over updates");
        assert_ne!(header_addr, updated_header_addr, "record revision should change after update");
        assert_eq!(updated_entry, Entry { field: Some("value".into()) }, "returned record should be changed after update");
        let (header_addr_3, returned_address_3, third_entry) = read_record_entry::<Entry, EntryWithIdentity, EntryId,_,_>(&entry_type, &identity_address).unwrap();
        assert_eq!(base_address.as_ref(), returned_address_3.as_ref(), "record should have consistent ID over updates");
        assert_eq!(header_addr_3, updated_header_addr, "record revision should be same as latest update");
        assert_eq!(third_entry, Entry { field: Some("value".into()) }, "retrieved record should be changed after update");

        // DELETE
        let _ = delete_record::<Entry>(&updated_header_addr);

        // Verify read failure
        let _failure = read_record_entry::<Entry, EntryWithIdentity, EntryId,_,_>(&entry_type, &identity_address).err().unwrap();
    }
}
