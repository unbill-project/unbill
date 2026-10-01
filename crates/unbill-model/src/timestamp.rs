use std::fmt;

use autosurgeon::{HydrateError, Prop, ReadDoc, Reconciler};

/// Unix timestamp in milliseconds.
///
/// A thin newtype over `i64` that makes datetime fields explicit in the type
/// system and prevents accidentally passing arbitrary integers where a time
/// value is expected.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct Timestamp(i64);

impl Timestamp {
    // sirno:witness:unbill-model:begin
    /// Current wall-clock time as a `Timestamp`.
    ///
    /// Returns an error if the system clock is before the Unix epoch.
    pub fn now() -> Result<Self, web_time::SystemTimeError> {
        Self::from_system_time(web_time::SystemTime::now())
    }

    fn from_system_time(time: web_time::SystemTime) -> Result<Self, web_time::SystemTimeError> {
        let millis = time.duration_since(web_time::UNIX_EPOCH)?.as_millis() as i64;
        Ok(Self(millis))
    }
    // sirno:witness:unbill-model:end

    pub fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    pub fn as_millis(self) -> i64 {
        self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ISO-8601-ish: seconds since epoch, millisecond precision.
        // Full calendar formatting can be added when a datetime crate is introduced.
        write!(f, "{}.{:03}s", self.0 / 1000, self.0.abs() % 1000)
    }
}

// --- autosurgeon integration ---
// Delegate Reconcile / Hydrate to i64 so Timestamp is transparent in Automerge docs.

impl autosurgeon::Reconcile for Timestamp {
    type Key<'a> = autosurgeon::reconcile::NoKey;

    fn reconcile<R: Reconciler>(&self, reconciler: R) -> Result<(), R::Error> {
        self.0.reconcile(reconciler)
    }
}

impl autosurgeon::Hydrate for Timestamp {
    fn hydrate<'a, D: ReadDoc>(
        doc: &'a D,
        obj: &automerge::ObjId,
        prop: Prop<'a>,
    ) -> Result<Self, HydrateError> {
        i64::hydrate(doc, obj, prop).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_ordering() {
        let t1 = Timestamp::from_millis(1000);
        let t2 = Timestamp::from_millis(2000);
        assert!(t1 < t2);
        assert!(t2 > t1);
        assert_eq!(t1, Timestamp::from_millis(1000));
    }

    #[test]
    fn test_timestamp_round_trip_millis() {
        let millis = 1_700_000_000_000i64;
        assert_eq!(Timestamp::from_millis(millis).as_millis(), millis);
    }

    #[test]
    fn test_timestamp_now_is_positive() -> Result<(), web_time::SystemTimeError> {
        assert!(Timestamp::now()?.as_millis() > 0);
        Ok(())
    }

    #[test]
    fn test_unix_epoch_is_zero() -> Result<(), web_time::SystemTimeError> {
        assert_eq!(
            Timestamp::from_system_time(web_time::UNIX_EPOCH)?,
            Timestamp::from_millis(0)
        );
        Ok(())
    }

    #[test]
    fn test_system_time_preserves_milliseconds() -> Result<(), Box<dyn std::error::Error>> {
        let time = web_time::UNIX_EPOCH
            .checked_add(std::time::Duration::from_millis(1234))
            .ok_or("test timestamp must be representable")?;
        assert_eq!(
            Timestamp::from_system_time(time)?,
            Timestamp::from_millis(1234)
        );
        Ok(())
    }

    #[test]
    fn test_time_before_unix_epoch_returns_error() -> Result<(), Box<dyn std::error::Error>> {
        let time = web_time::UNIX_EPOCH
            .checked_sub(std::time::Duration::from_millis(1))
            .ok_or("test timestamp must be representable")?;
        let error = Timestamp::from_system_time(time)
            .err()
            .ok_or("a pre-epoch timestamp must be rejected")?;
        assert_eq!(error.duration(), std::time::Duration::from_millis(1));
        Ok(())
    }

    #[test]
    fn test_timestamp_display() {
        let t = Timestamp::from_millis(1_000_500);
        assert_eq!(format!("{t}"), "1000.500s");
    }
}
