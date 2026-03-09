use anyhow::Context;
use sqlx::{FromRow, SqlitePool};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    connection::{Connection, ConnectionExt},
    types::Key,
};

/// The number of failed login attempts allowed before locking out an account.
const LOCKOUT_THRESHOLD: i64 = 10;

/// The time window (in seconds) for counting failed attempts.
const LOCKOUT_WINDOW_SECS: i64 = 300; // 5 minutes

/// Represents a login attempt record for brute force protection.
#[derive(Debug, FromRow)]
pub struct LoginAttempt {
    pub id: Key<LoginAttempt>,
    pub username: String,
    pub ip_address: Option<String>,
    pub attempted_at: OffsetDateTime,
    pub success: bool,
}

impl LoginAttempt {
    /// Record a login attempt (success or failure).
    ///
    /// This inserts a new record with a generated UUID. The attempt is logged
    /// via tracing for audit purposes.
    pub async fn record(
        connection: &Connection,
        username: &str,
        ip_address: Option<&str>,
        success: bool,
    ) -> anyhow::Result<()> {
        let id = Key::<LoginAttempt>::new();
        let now = OffsetDateTime::now_utc();

        let result = connection
            .execute(
                "INSERT INTO login_attempts (id, username, ip_address, attempted_at, success) \
            VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    id,
                    String::from(username),
                    ip_address.map(String::from),
                    now.format(&Rfc3339).context("failed to format date")?,
                    success,
                ),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to record login attempt");
        }

        tracing::info!(
            %username,
            ip_address = ip_address.unwrap_or("-"),
            success,
            "Login attempt recorded"
        );

        Ok(())
    }

    /// Check if an account is currently locked out due to too many failed attempts.
    ///
    /// Returns `true` if there have been `LOCKOUT_THRESHOLD` or more failed attempts
    /// within the last `LOCKOUT_WINDOW_SECS` seconds.
    pub async fn is_locked_out(connection: &Connection, username: &str) -> anyhow::Result<bool> {
        let cutoff = OffsetDateTime::now_utc() - time::Duration::seconds(LOCKOUT_WINDOW_SECS);

        let count: i64 = connection
            .query_scalar(
                "SELECT COUNT(*) FROM login_attempts \
                WHERE username = ?1 AND attempted_at > ?2 AND success = 0",
                (
                    String::from(username),
                    cutoff.format(&Rfc3339).context("failed to format date")?,
                ),
            )
            .await?;

        let locked = count >= LOCKOUT_THRESHOLD;

        if locked {
            tracing::warn!(
                %username,
                failed_attempts = count,
                threshold = LOCKOUT_THRESHOLD,
                window_secs = LOCKOUT_WINDOW_SECS,
                "Account is locked out due to too many failed attempts"
            );
        }

        Ok(locked)
    }
}
