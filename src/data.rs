// Copyright 2020 ADVANCA PTE. LTD.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::fmt;
use crate::{
    de, Deserialize, Deserializer, SeqAccess, Serialize, SerializeTuple, Serializer, Visitor,
};
use crate::{vec, vec::Vec};

const PADDING_VALUE: u8 = 255;

/// Type of stored data
pub type Data = Vec<u8>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataWrapper {
    /// The actual data buffer
    pub buf: Data,
    /// The expected maximum length of `buf`.
    ///
    /// If no sufficent data is in `buf`, padding will be added during serialization
    pub max_len: usize,
}

impl Serialize for DataWrapper {
    /// Serialize function
    ///
    /// The field `max_len` is not serialized as it becomes padding bytes.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let padding = vec![PADDING_VALUE as u8; self.max_len - self.buf.len()];
        let mut s = serializer.serialize_tuple(2)?;
        s.serialize_element(&self.buf)?;
        s.serialize_element(&padding)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for DataWrapper {
    /// Deserilize function
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DataVisitor;

        impl<'de> Visitor<'de> for DataVisitor {
            type Value = DataWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("DataWrapper")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<DataWrapper, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let buf: Data = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                //XXX: we could skip this field by utilizing the following snippet
                // ```
                // while let Some(IgnoredAny) = seq.next_element()? {
                //     // Ignore rest
                // }
                // ```
                // However, Bincode doesn't support this
                let padding: Data = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                let max_len = buf.len() + padding.len();
                Ok(DataWrapper { buf, max_len })
            }
        }

        deserializer.deserialize_tuple(2, DataVisitor)
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{DataWrapper, PADDING_VALUE};
    use serde_test::{assert_de_tokens, assert_ser_tokens, Token};

    #[test]
    fn test_ser_empty() {
        let data = DataWrapper {
            buf: vec![],
            max_len: 4,
        };

        assert_ser_tokens(
            &data,
            &[
                Token::Tuple { len: 2 },
                Token::Seq { len: Some(0) },
                Token::SeqEnd,
                Token::Seq { len: Some(4) },
                Token::U8(PADDING_VALUE),
                Token::U8(PADDING_VALUE),
                Token::U8(PADDING_VALUE),
                Token::U8(PADDING_VALUE),
                Token::SeqEnd,
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn test_de_empty() {
        let data = DataWrapper {
            buf: vec![],
            max_len: 4,
        };

        assert_de_tokens(
            &data,
            &[
                Token::Tuple { len: 2 },
                Token::Seq { len: Some(0) },
                Token::SeqEnd,
                Token::Seq { len: Some(4) },
                Token::U8(PADDING_VALUE),
                Token::U8(PADDING_VALUE),
                Token::U8(PADDING_VALUE),
                Token::U8(PADDING_VALUE),
                Token::SeqEnd,
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn test_ser_partial() {
        let data = DataWrapper {
            buf: vec![1, 2],
            max_len: 4,
        };

        assert_ser_tokens(
            &data,
            &[
                Token::Tuple { len: 2 },
                Token::Seq { len: Some(2) },
                Token::U8(1),
                Token::U8(2),
                Token::SeqEnd,
                Token::Seq { len: Some(2) },
                Token::U8(PADDING_VALUE),
                Token::U8(PADDING_VALUE),
                Token::SeqEnd,
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn test_de_partial() {
        let data = DataWrapper {
            buf: vec![1, 2],
            max_len: 4,
        };

        assert_de_tokens(
            &data,
            &[
                Token::Tuple { len: 2 },
                Token::Seq { len: Some(2) },
                Token::U8(1),
                Token::U8(2),
                Token::SeqEnd,
                Token::Seq { len: Some(2) },
                Token::U8(PADDING_VALUE),
                Token::U8(PADDING_VALUE),
                Token::SeqEnd,
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn test_ser_full() {
        let data = DataWrapper {
            buf: vec![1, 2, 3, 4],
            max_len: 4,
        };

        assert_ser_tokens(
            &data,
            &[
                Token::Tuple { len: 2 },
                Token::Seq { len: Some(4) },
                Token::U8(1),
                Token::U8(2),
                Token::U8(3),
                Token::U8(4),
                Token::SeqEnd,
                Token::Seq { len: Some(0) },
                Token::SeqEnd,
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn test_de_full() {
        let data = DataWrapper {
            buf: vec![1, 2, 3, 4],
            max_len: 4,
        };

        assert_de_tokens(
            &data,
            &[
                Token::Tuple { len: 2 },
                Token::Seq { len: Some(4) },
                Token::U8(1),
                Token::U8(2),
                Token::U8(3),
                Token::U8(4),
                Token::SeqEnd,
                Token::Seq { len: Some(0) },
                Token::SeqEnd,
                Token::TupleEnd,
            ],
        );
    }
}
