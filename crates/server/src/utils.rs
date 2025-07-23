use poem::session::Session;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

pub trait SessionExt {
    fn take<T: DeserializeOwned>(&self, name: &str) -> Option<T>;
}

impl SessionExt for Session {
    fn take<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        let value = self.get(name)?;
        self.remove(name);
        Some(value)
    }
}

pub fn validate_slug(slug: &str) -> Result<(), ValidationError> {
    if slug
        .chars()
        .any(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
    {
        return Err(ValidationError::new("invalid_slug").with_message("Invalid URL slug".into()));
    }

    Ok(())
}

pub trait ValidationErrorsExt {
    fn merge(&mut self, other: ValidationErrors);
}

impl ValidationErrorsExt for ValidationErrors {
    fn merge(&mut self, other: ValidationErrors) {
        for (field, errors) in other.0 {
            if let ValidationErrorsKind::Field(errors) = errors {
                for error in errors {
                    if let ValidationErrorsKind::Field(ref mut own) = self
                        .0
                        .entry(field.clone())
                        .or_insert_with(|| ValidationErrorsKind::Field(vec![]))
                    {
                        own.push(error);
                    } else {
                        panic!("Attempt to merge non-field errors into field errors");
                    }
                }
            } else {
                panic!("Attempt to add non-field errors to field errors");
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeUnit {
    B,
    KB,
    MB,
    GB,
    TB,
}

impl SizeUnit {
    pub fn to_bytes(self) -> i64 {
        match self {
            SizeUnit::B => 1,
            SizeUnit::KB => 1_000,
            SizeUnit::MB => 1_000_000,
            SizeUnit::GB => 1_000_000_000,
            SizeUnit::TB => 1_000_000_000_000,
        }
    }
}
