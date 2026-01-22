use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

use crate::types::Key;

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
        pool: &SqlitePool,
        username: &str,
        ip_address: Option<&str>,
        success: bool,
    ) -> sqlx::Result<()> {
        let id = Key::<LoginAttempt>::new();
        let now = OffsetDateTime::now_utc();

        sqlx::query(
            "INSERT INTO login_attempts (id, username, ip_address, attempted_at, success) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(username)
        .bind(ip_address)
        .bind(now)
        .bind(success)
        .execute(pool)
        .await?;

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
    pub async fn is_locked_out(pool: &SqlitePool, username: &str) -> sqlx::Result<bool> {
        let cutoff = OffsetDateTime::now_utc() - time::Duration::seconds(LOCKOUT_WINDOW_SECS);

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM login_attempts \
             WHERE username = $1 AND attempted_at > $2 AND success = 0",
        )
        .bind(username)
        .bind(cutoff)
        .fetch_one(pool)
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
