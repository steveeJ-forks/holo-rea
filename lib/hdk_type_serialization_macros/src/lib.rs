/**
 * Type aliases used to ensure explicit awareness of applicable record types in VF structs
 *
 * To convert wrapped values to an `EntryHash`, use `aliased_val.as_ref()`.
 * To convert a plain `EntryHash` to its wrapped form, use `raw_address.into()`.
 */
pub use std::convert::TryFrom;
pub use holochain_serialized_bytes::prelude::*;
pub use holo_hash::{DnaHash, AnyDhtHash};

#[macro_export]
macro_rules! newtype_wrapper {
    ($id:ident => $base:ty) => {
        impl From<$id> for $base {
            fn from(v: $id) -> $base {
                v.0
            }
        }

        impl From<$base> for $id {
            fn from (v: $base) -> $id {
                $id(v)
            }
        }

        impl AsRef<$base> for $id {
            fn as_ref(&self) -> &$base {
                &self.0
            }
        }
    }
}

#[macro_export]
macro_rules! simple_alias {
    ($id:ident => $base:ty) => {
        #[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq)]
        pub struct $id(pub $base);

        newtype_wrapper!($id => $base);
    }
}

#[macro_export]
macro_rules! addressable_identifier {
    ($id:ident, $r:ident => $base:ty) => {
        // internal wrapped newtype suitable for use within this cell, without DnaHash of cell
        #[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq)]
        pub struct $id(pub $base);

        newtype_wrapper!($id => $base);

        // externally facing type, with DnaHash of cell for context
        #[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq)]
        pub struct $r(pub DnaHash, pub $base);

        // convert wrapped type to externally facing type
        impl From<(DnaHash, $id)> for $r {
            fn from(v: (DnaHash, $id)) -> Self {
                Self(v.0, v.1.0)
            }
        }

        // extract wrapped cell-local identifier from externally facing type
        impl From<$r> for $id {
            fn from(v: $r) -> Self {
                Self(v.1)
            }
        }

        // reference raw cell-local identifier from externally facing type
        impl AsRef<$base> for $r {
            fn as_ref(&self) -> &$base {
                &self.1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use holo_hash::HOLO_HASH_UNTYPED_LEN;

    #[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq)]
    pub struct SomeValue(pub String);

    #[test]
    fn test_addressable_type() {
        addressable_identifier!(Ident => SomeValue);

        let base = SomeValue("test".to_string());
        let wrapped: Ident = base.clone().into();
        let external: IdentRemote = (DnaHash::from_raw_36(vec![0xdb; HOLO_HASH_UNTYPED_LEN]), wrapped).into();
        let extracted: SomeValue = external.into();

        assert_eq!(base, extracted, "Original data matches wrapped, externalised, extracted roundtrip data");
    }
}
