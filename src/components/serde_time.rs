//! Custom serde serialization for time types
//!
//! Provides serialization support for std::time::Instant which doesn't implement
//! Serialize/Deserialize by default.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serialize an Instant as nanoseconds since epoch
pub fn serialize_instant<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Convert Instant to SystemTime for serialization
    // This is an approximation since Instant is not tied to wall clock time
    let now_instant = Instant::now();
    let now_system = SystemTime::now();

    let duration_since_now = if *instant > now_instant {
        instant.duration_since(now_instant)
    } else {
        now_instant.duration_since(*instant)
    };

    let system_time = if *instant > now_instant {
        now_system + duration_since_now
    } else {
        now_system - duration_since_now
    };

    system_time
        .duration_since(UNIX_EPOCH)
        .map_err(serde::ser::Error::custom)?
        .as_nanos()
        .serialize(serializer)
}

/// Deserialize an Instant from nanoseconds since epoch
pub fn deserialize_instant<'de, D>(deserializer: D) -> Result<Instant, D::Error>
where
    D: Deserializer<'de>,
{
    let nanos = u128::deserialize(deserializer)?;
    let system_time = UNIX_EPOCH + Duration::from_nanos(nanos as u64);

    // Convert back to Instant (approximation)
    let now_system = SystemTime::now();
    let now_instant = Instant::now();

    let duration_since_now = if system_time > now_system {
        system_time
            .duration_since(now_system)
            .unwrap_or(Duration::ZERO)
    } else {
        now_system
            .duration_since(system_time)
            .unwrap_or(Duration::ZERO)
    };

    Ok(if system_time > now_system {
        now_instant + duration_since_now
    } else {
        now_instant - duration_since_now
    })
}

/// Serialize an Option<Instant>
pub fn serialize_instant_option<S>(
    instant: &Option<Instant>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match instant {
        Some(instant) => serialize_instant(instant, serializer),
        None => serializer.serialize_none(),
    }
}

/// Deserialize an Option<Instant>
pub fn deserialize_instant_option<'de, D>(deserializer: D) -> Result<Option<Instant>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<u128>::deserialize(deserializer)?.map(|nanos| {
        let system_time = UNIX_EPOCH + Duration::from_nanos(nanos as u64);
        let now_system = SystemTime::now();
        let now_instant = Instant::now();

        let duration_since_now = if system_time > now_system {
            system_time
                .duration_since(now_system)
                .unwrap_or(Duration::ZERO)
        } else {
            now_system
                .duration_since(system_time)
                .unwrap_or(Duration::ZERO)
        };

        if system_time > now_system {
            now_instant + duration_since_now
        } else {
            now_instant - duration_since_now
        }
    }))
}
