use compact_str::CompactString;
use std::borrow::Borrow;
use std::hash::Hash;
use std::{io, io::Write};

/// Error when reaching the static capacity limit of a log or log group metadata.
#[derive(Debug, thiserror::Error)]
#[error("reached capacity limit")]
pub struct CapacityError(pub CompactString, pub CompactString);

/// Log entry with a timestamp and fixed capacity key-value pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Log<const N_KEY_PAIR: usize = 8> {
    /// UNIX Time Format
    timestamp: u32,
    /// for time nano part
    subsec_nanosecond: Option<u32>,
    /// log contents key value pairs
    contents: heapless::FnvIndexMap<CompactString, CompactString, N_KEY_PAIR>,
}

/// Metadata for a group of logs, including topic, source, and fixed capacity key-value tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogGroupMetadata<const N_TAGS: usize = 8> {
    topic: Option<CompactString>,
    source: Option<CompactString>,
    log_tags: heapless::FnvIndexMap<CompactString, CompactString, N_TAGS>,
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

impl<const N_KEY_PAIR: usize> Log<N_KEY_PAIR> {
    /// Create a new log with the current timestamp.
    pub fn now() -> Self {
        let now = jiff::Timestamp::now();
        Log {
            timestamp: now.as_second() as u32,
            subsec_nanosecond: Some(now.subsec_nanosecond() as u32),
            contents: heapless::FnvIndexMap::new(),
        }
    }

    /// Create a new log with the specified timestamp and optional subsecond nanosecond.
    pub fn new(timestamp: u32, subsec_nanosecond: Option<u32>) -> Self {
        Log {
            timestamp,
            subsec_nanosecond,
            contents: heapless::FnvIndexMap::new(),
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
        self.try_with(key, value).ok();
        self
    }

    /// Add a key-value pair to the log contents.
    pub fn try_with(
        &mut self,
        key: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Result<&mut Self, CapacityError> {
        self.contents
            .insert(key.into(), value.into())
            .map_err(|(k, v)| CapacityError(k, v))?;
        Ok(self)
    }

    /// Remove a key-value pair from the log contents.
    pub fn remove<Q>(&mut self, key: &Q)
    where
        CompactString: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.contents.remove(key);
    }
}

impl<const N_TAGS: usize> LogGroupMetadata<N_TAGS> {
    /// Create a new log group metadata with default values.
    pub fn new() -> Self {
        LogGroupMetadata {
            topic: None,
            source: None,
            log_tags: heapless::FnvIndexMap::new(),
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
        self.try_with_tag(key, value).ok();
        self
    }

    /// Add a tag to the log group metadata.
    pub fn try_with_tag(
        &mut self,
        key: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Result<&mut Self, CapacityError> {
        self.log_tags
            .insert(key.into(), value.into())
            .map_err(|(k, v)| CapacityError(k, v))?;
        Ok(self)
    }

    /// Remove a tag from the log group metadata.
    pub fn remove_tag<Q>(&mut self, key: &Q)
    where
        CompactString: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.log_tags.remove(key);
    }
}

// Manual implementation for faster encoding
pub(crate) fn encode_log_group<const N_TAGS: usize, const N_KEY_PAIR: usize>(
    writer: &mut impl Write,
    metadata: &LogGroupMetadata<N_TAGS>,
    logs: &[Log<N_KEY_PAIR>],
) -> io::Result<()> {
    for log in logs {
        encode_message(1u32, log, writer)?;
    }
    if let Some(ref value) = metadata.topic {
        encode_str(3u32, value, writer)?;
    }
    if let Some(ref value) = metadata.source {
        encode_str(4u32, value, writer)?;
    }
    for tag in &metadata.log_tags {
        encode_message(6u32, &tag, writer)?;
    }

    Ok(())
}

pub(crate) fn calc_log_group_encoded_len<const N_TAGS: usize, const N_KEY_PAIR: usize>(
    metadata: &LogGroupMetadata<N_TAGS>,
    logs: &[Log<N_KEY_PAIR>],
) -> usize {
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
    fn encode(&self, writer: &mut impl Write) -> io::Result<()>;
    fn encoded_len(&self) -> usize;
}

impl<T: Message> Message for &T {
    #[inline]
    fn encode(&self, writer: &mut impl Write) -> io::Result<()> {
        T::encode(self, writer)
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        T::encoded_len(self)
    }
}

impl<S: AsRef<str>> Message for (S, S) {
    #[inline]
    fn encode(&self, writer: &mut impl Write) -> io::Result<()> {
        encode_str(1u32, self.0.as_ref(), writer)?;
        encode_str(2u32, self.1.as_ref(), writer)
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        encoded_str_len(1u32, self.0.as_ref()) + encoded_str_len(2u32, self.1.as_ref())
    }
}

impl<const N_KEY_PAIR: usize> Message for Log<N_KEY_PAIR> {
    #[inline]
    fn encode(&self, writer: &mut impl Write) -> io::Result<()> {
        encode_varint_field(1u32, self.timestamp as u64, writer)?;
        for msg in &self.contents {
            encode_message(2u32, &msg, writer)?;
        }
        if let Some(value) = self.subsec_nanosecond {
            encode_fixed32(4u32, value, writer)?;
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
fn encode_varint(mut value: u64, writer: &mut impl Write) -> io::Result<()> {
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
fn encode_key(tag: u32, wire_type: WireType, writer: &mut impl Write) -> io::Result<()> {
    let key = (tag << 3) | wire_type as u32;
    encode_varint(u64::from(key), writer)
}

#[inline]
fn encode_varint_field(tag: u32, value: u64, writer: &mut impl Write) -> io::Result<()> {
    encode_key(tag, WireType::Varint, writer)?;
    encode_varint(value, writer)
}

#[inline]
fn encode_fixed32(tag: u32, value: u32, writer: &mut impl Write) -> io::Result<()> {
    encode_key(tag, WireType::ThirtyTwoBit, writer)?;
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline]
fn encode_message(tag: u32, msg: &impl Message, writer: &mut impl Write) -> io::Result<()> {
    encode_key(tag, WireType::LengthDelimited, writer)?;
    encode_varint(msg.encoded_len() as u64, writer)?;
    msg.encode(writer)
}

#[inline]
fn encode_str(tag: u32, value: impl AsRef<str>, writer: &mut impl Write) -> io::Result<()> {
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
