use poem::session::Session;
use serde::de::DeserializeOwned;
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
                    self.add(field, error);
                }
            } else {
                panic!("Attempt to add non-field errors to field errors");
            }
        }
    }
}
