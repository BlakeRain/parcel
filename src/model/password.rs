use std::fmt::{Debug, Formatter};

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use pbkdf2::Pbkdf2;
use sqlx::{Database, Sqlite};

#[derive(Clone)]
pub enum StoredPassword {
    Pbkdf2(String),
    Argon2(String),
}

impl Debug for StoredPassword {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pbkdf2(_) => write!(f, "StoredPassword::Pbkdf2(...)"),
            Self::Argon2(_) => write!(f, "StoredPassword::Argon2(...)"),
        }
    }
}

impl TryFrom<&str> for StoredPassword {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with("$argon2") {
            Ok(StoredPassword::Argon2(value.to_string()))
        } else if value.starts_with("$pbkdf2") {
            Ok(StoredPassword::Pbkdf2(value.to_string()))
        } else {
            Err(anyhow::anyhow!("Unknown password hash type"))
        }
    }
}

impl sqlx::Type<Sqlite> for StoredPassword {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <String as sqlx::Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &<Sqlite as Database>::TypeInfo) -> bool {
        <String as sqlx::Type<Sqlite>>::compatible(ty)
    }
}

impl<'q> sqlx::Encode<'q, Sqlite> for StoredPassword {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            StoredPassword::Pbkdf2(password) => {
                sqlx::Encode::<Sqlite>::encode_by_ref(&password, buf)
            }
            StoredPassword::Argon2(password) => {
                sqlx::Encode::<Sqlite>::encode_by_ref(&password, buf)
            }
        }
    }
}

impl<'r> sqlx::Decode<'r, Sqlite> for StoredPassword {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        <String as sqlx::Decode<Sqlite>>::decode(value).map(|password| {
            if password.starts_with("$argon2") {
                StoredPassword::Argon2(password)
            } else if password.starts_with("$pbkdf2") {
                StoredPassword::Pbkdf2(password)
            } else {
                panic!("Unknown password hash type")
            }
        })
    }
}

impl StoredPassword {
    pub fn needs_migrating(&self) -> bool {
        matches!(self, Self::Pbkdf2(_))
    }

    pub fn new(plain: &str) -> anyhow::Result<Self> {
        let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(plain.as_bytes(), &salt)?.to_string();

        Ok(StoredPassword::Argon2(hash))
    }

    pub fn verify(&self, plain: &str) -> bool {
        match self {
            Self::Pbkdf2(hash) => Pbkdf2
                .verify_password(
                    plain.as_bytes(),
                    &PasswordHash::new(hash).expect("valid password hash"),
                )
                .is_ok(),

            Self::Argon2(hash) => {
                let parsed = PasswordHash::new(hash).expect("valid password hash");
                Argon2::default()
                    .verify_password(plain.as_bytes(), &parsed)
                    .is_ok()
            }
        }
    }
}
