use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::marker::PhantomData;
use serde::de::{Deserializer, Error, SeqAccess, Visitor};
use serde::ser::{SerializeTuple, Serializer};
use serde::{Deserialize, Serialize};

/// A data format exchanged by each peer to derive
/// the shared session key
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct PortalKeyExchange([u8; 33]);

/// A data format exchanged by each peer to confirm
/// that they have each derived the same key
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct PortalConfirmation([u8; 42]);

/// Provide a serde visitor to serialize/deserialize larger arrays
struct ArrayVisitor<T> {
    element: PhantomData<T>,
}

impl<'de, T, const N: usize> Visitor<'de> for ArrayVisitor<[T; N]>
where
    T: Default + Copy + Deserialize<'de>,
{
    type Value = [T; N];

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an array of length {}", N)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<[T; N], A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut arr = [T::default(); N];
        for i in 0..N {
            arr[i] = seq
                .next_element()?
                .ok_or_else(|| Error::invalid_length(i, &self))?;
        }
        Ok(arr)
    }
}

impl<'de> Deserialize<'de> for PortalKeyExchange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = ArrayVisitor {
            element: PhantomData,
        };
        let res =
            deserializer.deserialize_tuple(std::mem::size_of::<PortalKeyExchange>(), visitor)?;

        Ok(Self { 0: res })
    }
}

impl Serialize for PortalKeyExchange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_tuple(self.0.len())?;
        for elem in &self.0[..] {
            seq.serialize_element(elem)?;
        }
        seq.end()
    }
}

impl From<[u8; 33]> for PortalKeyExchange {
    fn from(s: [u8; 33]) -> Self {
        PortalKeyExchange { 0: s }
    }
}

impl<'a> Into<&'a [u8]> for &'a PortalKeyExchange {
    fn into(self) -> &'a [u8] {
        &self.0
    }
}

impl TryFrom<Vec<u8>> for PortalKeyExchange {
    type Error = &'static str;
    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self {
            0: v.try_into()
                .or(Err("Cannot convert into PortalKeyExchange"))?,
        })
    }
}

impl<'de> Deserialize<'de> for PortalConfirmation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = ArrayVisitor {
            element: PhantomData,
        };
        let res =
            deserializer.deserialize_tuple(std::mem::size_of::<PortalConfirmation>(), visitor)?;

        Ok(Self { 0: res })
    }
}

impl Serialize for PortalConfirmation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_tuple(self.0.len())?;
        for elem in &self.0[..] {
            seq.serialize_element(elem)?;
        }
        seq.end()
    }
}
