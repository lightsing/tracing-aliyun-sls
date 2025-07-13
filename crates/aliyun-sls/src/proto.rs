use compact_str::CompactString;
use std::borrow::Borrow;
use std::{io, io::Write};

cfg_if::cfg_if! {
    if #[cfg(all(feature = "inline-keypairs-16", not(feature = "inline-none")))] {
        pub(crate) const N_INLINE_KEY_PAIR: usize = 16;
    } else if #[cfg(all(feature = "inline-keypairs-8", not(feature = "inline-none")))] {
        pub(crate) const N_INLINE_KEY_PAIR: usize = 8;
    } else if #[cfg(all(feature = "inline-keypairs-4", not(feature = "inline-none")))] {
        pub(crate) const N_INLINE_KEY_PAIR: usize = 4;
    } else if #[cfg(all(feature = "inline-keypairs-2", not(feature = "inline-none")))] {
        pub(crate) const N_INLINE_KEY_PAIR: usize = 2;
    } else {
        pub(crate) const N_INLINE_KEY_PAIR: usize = 0;
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(feature = "inline-tags-16", not(feature = "inline-none")))] {
        pub(crate) const N_INLINE_TAGS: usize = 16;
    } else if #[cfg(all(feature = "inline-tags-8", not(feature = "inline-none")))] {
        pub(crate) const N_INLINE_TAGS: usize = 8;
    } else if #[cfg(all(feature = "inline-tags-4", not(feature = "inline-none")))] {
        pub(crate) const N_INLINE_TAGS: usize = 4;
    } else if #[cfg(all(feature = "inline-tags-2", not(feature = "inline-none")))] {
        pub(crate) const N_INLINE_TAGS: usize = 2;
    } else {
        pub(crate) const N_INLINE_TAGS: usize = 0;
    }
}

type Map<const N: usize> = litemap::LiteMap<
    CompactString,
    CompactString,
    smallvec::SmallVec<(CompactString, CompactString), N>,
>;

/// Log entry with a timestamp and fixed capacity key-value pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Log {
    /// UNIX Time Format
    timestamp: u32,
    /// for time nano part
    subsec_nanosecond: Option<u32>,
    /// log contents key value pairs
    contents: Map<N_INLINE_KEY_PAIR>,
}

/// Metadata for a group of logs, including topic, source, and fixed capacity key-value tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogGroupMetadata {
    topic: Option<CompactString>,
    source: Option<CompactString>,
    log_tags: Map<N_INLINE_TAGS>,
}

impl Default for Log {
    fn default() -> Self {
        Log::now()
    }
}

impl Default for LogGroupMetadata {
    fn default() -> Self {
        LogGroupMetadata::new()
    }
}

impl Log {
    /// Create a new log with the current timestamp.
    pub fn now() -> Self {
        let now = jiff::Timestamp::now();
        Log {
            timestamp: now.as_second() as u32,
            subsec_nanosecond: Some(now.subsec_nanosecond() as u32),
            contents: Map::new(),
        }
    }

    /// Create a new log with the specified timestamp and optional subsecond nanosecond.
    pub fn new(timestamp: u32, subsec_nanosecond: Option<u32>) -> Self {
        Log {
            timestamp,
            subsec_nanosecond,
            contents: Map::new(),
        }
    }

    /// Modify the timestamp of the log.
    pub fn with_timestamp(mut self, timestamp: u32) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Modify the subsecond nanosecond of the log.
    pub fn with_subsec_nanosecond(mut self, subsec_nanosecond: u32) -> Self {
        self.subsec_nanosecond = Some(subsec_nanosecond);
        self
    }

    /// Add a key-value pair to the log contents.
    pub fn with(mut self, key: impl Into<CompactString>, value: impl Into<CompactString>) -> Self {
        self.contents.insert(key.into(), value.into());
        self
    }

    /// Remove a key-value pair from the log contents.
    pub fn remove<Q>(&mut self, key: &Q)
    where
        CompactString: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.contents.remove(key);
    }
}

impl LogGroupMetadata {
    /// Create a new log group metadata with default values.
    pub fn new() -> Self {
        LogGroupMetadata {
            topic: None,
            source: None,
            log_tags: Map::new(),
        }
    }

    /// Set the topic for the log group metadata.
    pub fn with_topic(mut self, topic: impl Into<CompactString>) -> Self {
        self.topic = Some(topic.into());
        self
    }

    /// Set the source for the log group metadata.
    pub fn with_source(mut self, source: impl Into<CompactString>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add a tag to the log group metadata.
    pub fn with_tag(
        mut self,
        key: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Self {
        self.log_tags.insert(key.into(), value.into());
        self
    }

    /// Remove a tag from the log group metadata.
    pub fn remove_tag<Q>(&mut self, key: &Q)
    where
        CompactString: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.log_tags.remove(key);
    }
}

// Manual implementation for faster encoding
pub(crate) fn encode_log_group<W: Write>(
    writer: &mut W,
    metadata: &LogGroupMetadata,
    logs: &[Log],
) -> io::Result<()> {
    for log in logs.as_ref() {
        encode_message(1u32, log, writer)?;
    }
    if let Some(ref value) = metadata.topic {
        encode_str(3u32, value, writer)?;
    }
    if let Some(ref value) = metadata.source {
        encode_str(4u32, value, writer)?;
    }
    for tag in metadata.log_tags.iter() {
        encode_message(6u32, &tag, writer)?;
    }

    Ok(())
}

pub(crate) fn calc_log_group_encoded_len(metadata: &LogGroupMetadata, logs: &[Log]) -> usize {
    let logs = logs.as_ref();
    encoded_len_repeated(1u32, logs.iter(), logs.len())
        + metadata
            .topic
            .as_ref()
            .map_or(0, |value| encoded_str_len(3u32, value))
        + metadata
            .source
            .as_ref()
            .map_or(0, |value| encoded_str_len(4u32, value))
        + encoded_len_repeated(6u32, metadata.log_tags.iter(), metadata.log_tags.len())
}

trait Message {
    fn encode_into_vec<W: Write>(&self, writer: &mut W) -> io::Result<()>;
    fn encoded_len(&self) -> usize;
}

impl<T: Message> Message for &T {
    #[inline]
    fn encode_into_vec<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        T::encode_into_vec(self, writer)
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        T::encoded_len(self)
    }
}

impl<S: AsRef<str>> Message for (S, S) {
    #[inline]
    fn encode_into_vec<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        encode_str(1u32, self.0.as_ref(), writer)?;
        encode_str(2u32, self.1.as_ref(), writer)
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        encoded_str_len(1u32, self.0.as_ref()) + encoded_str_len(2u32, self.1.as_ref())
    }
}

impl Message for Log {
    #[inline]
    fn encode_into_vec<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        encode_varint_field(1u32, self.timestamp as u64, writer).expect("infallible");
        for msg in &self.contents {
            encode_message(2u32, &msg, writer).expect("infallible");
        }
        if let Some(value) = self.subsec_nanosecond {
            encode_fixed32(4u32, value, writer).expect("infallible");
        }
        Ok(())
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        encoded_varint_field_len(1u32, self.timestamp as u64)
            + encoded_len_repeated(2u32, self.contents.iter(), self.contents.len())
            + self
                .subsec_nanosecond
                .as_ref()
                .map_or(0, |_| encoded_fixed32_len(4u32))
    }
}

// Copy from prost

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum WireType {
    Varint = 0,
    SixtyFourBit = 1,
    LengthDelimited = 2,
    StartGroup = 3,
    EndGroup = 4,
    ThirtyTwoBit = 5,
}

#[inline]
fn encode_varint<W: Write>(mut value: u64, writer: &mut W) -> io::Result<()> {
    loop {
        if value < 0x80 {
            writer.write_all(&[value as u8])?;
            break;
        } else {
            writer.write_all(&[((value & 0x7F) | 0x80) as u8])?;
            value >>= 7;
        }
    }
    Ok(())
}

#[inline]
fn encode_key<W: Write>(tag: u32, wire_type: WireType, writer: &mut W) -> io::Result<()> {
    let key = (tag << 3) | wire_type as u32;
    encode_varint(u64::from(key), writer)
}

#[inline]
fn encode_varint_field<W: Write>(tag: u32, value: u64, writer: &mut W) -> io::Result<()> {
    encode_key(tag, WireType::Varint, writer)?;
    encode_varint(value, writer)
}

#[inline]
fn encode_fixed32<W: Write>(tag: u32, value: u32, writer: &mut W) -> io::Result<()> {
    encode_key(tag, WireType::ThirtyTwoBit, writer)?;
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline]
fn encode_message<W: Write>(tag: u32, msg: &impl Message, writer: &mut W) -> io::Result<()> {
    encode_key(tag, WireType::LengthDelimited, writer)?;
    encode_varint(msg.encoded_len() as u64, writer)?;
    msg.encode_into_vec(writer)
}

#[inline]
fn encode_str<W: Write>(tag: u32, value: impl AsRef<str>, writer: &mut W) -> io::Result<()> {
    let value = value.as_ref();
    encode_key(tag, WireType::LengthDelimited, writer)?;
    encode_varint(value.len() as u64, writer)?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

#[inline]
fn encoded_len_varint(value: u64) -> usize {
    // Based on [VarintSize64][1].
    // [1]: https://github.com/google/protobuf/blob/3.3.x/src/google/protobuf/io/coded_stream.h#L1301-L1309
    ((((value | 1).leading_zeros() ^ 63) * 9 + 73) / 64) as usize
}

#[inline]
fn key_len(tag: u32) -> usize {
    encoded_len_varint(u64::from(tag << 3))
}

#[inline]
fn encoded_str_len(tag: u32, value: impl AsRef<str>) -> usize {
    let value = value.as_ref();
    key_len(tag) + encoded_len_varint(value.len() as u64) + value.len()
}

#[inline]
fn encoded_len_repeated<I, M>(tag: u32, messages: I, len: usize) -> usize
where
    I: Iterator<Item = M>,
    M: Message,
{
    key_len(tag) * len
        + messages
            .map(|m| m.encoded_len())
            .map(|len| len + encoded_len_varint(len as u64))
            .sum::<usize>()
}

#[inline]
fn encoded_varint_field_len(tag: u32, value: u64) -> usize {
    key_len(tag) + encoded_len_varint(value)
}

#[inline]
fn encoded_fixed32_len(tag: u32) -> usize {
    key_len(tag) + 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size() {
        println!("size_of::<Log>() = {}", size_of::<Log>());
        println!(
            "size_of::<LogGroupMetadata>() = {}",
            size_of::<LogGroupMetadata>()
        );
    }
}
