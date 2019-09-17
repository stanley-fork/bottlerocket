use crate::decoded::{Decoded, Hex};
use crate::error;
use crate::key::Key;
use serde::{de::Error as _, Deserializer};
use snafu::ensure;
use std::collections::HashMap;
use std::fmt;

/// Validates the key ID for each key during deserialization and fails if any don't match.
pub(crate) fn deserialize_keys<'de, D>(
    deserializer: D,
) -> Result<HashMap<Decoded<Hex>, Key>, D::Error>
where
    D: Deserializer<'de>,
{
    // An inner function that does actual key ID validation:
    // * fails if a key ID doesn't match its contents
    // * fails if there is a duplicate key ID
    // If this passes we insert the entry.
    fn validate_and_insert_entry(
        keyid: Decoded<Hex>,
        key: Key,
        map: &mut HashMap<Decoded<Hex>, Key>,
    ) -> Result<(), error::Error> {
        let calculated = key.key_id()?;
        ensure!(
            keyid == calculated,
            error::HashMismatch {
                context: "key".to_owned(),
                calculated: hex::encode(&calculated),
                expected: hex::encode(&keyid),
            }
        );
        let keyid_hex = hex::encode(&keyid); // appease borrowck
        ensure!(
            map.insert(keyid, key).is_none(),
            error::DuplicateKeyId { keyid: keyid_hex }
        );
        Ok(())
    }

    // The rest of this is fitting the above function into serde and doing error type conversion.
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = HashMap<Decoded<Hex>, Key>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut map = HashMap::new();
            while let Some((keyid, key)) = access.next_entry()? {
                validate_and_insert_entry(keyid, key, &mut map).map_err(M::Error::custom)?;
            }
            Ok(map)
        }
    }

    deserializer.deserialize_map(Visitor)
}

#[cfg(test)]
mod tests {
    use crate::{Root, Signed};

    #[test]
    fn duplicate_keyid() {
        assert!(serde_json::from_str::<Signed<Root>>(include_str!(
            "../tests/data/duplicate-keyid/root.json"
        ))
        .is_err());
    }
}
