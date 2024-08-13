use minijinja::{
    value::{Kwargs, ValueKind},
    Environment, Error, ErrorKind, Value,
};
use serde::{de::value::SeqDeserializer, Deserialize};
use time::{
    format_description::{well_known::Iso8601, FormatItem},
    macros::format_description,
    Date, OffsetDateTime, PrimitiveDateTime,
};

enum ParsedDate {
    Date(Date),
    DateTime(OffsetDateTime),
}

// Parse an `OffsetDateTime` from a string.
fn parse_datetime_string(input: &str) -> Result<ParsedDate, Error> {
    match OffsetDateTime::parse(input, &Iso8601::PARSING) {
        Ok(datetime) => Ok(ParsedDate::DateTime(datetime)),
        Err(err) => match PrimitiveDateTime::parse(input, &Iso8601::PARSING) {
            Ok(datetime) => Ok(ParsedDate::DateTime(datetime.assume_utc())),
            Err(_) => match Date::parse(input, &Iso8601::PARSING) {
                Ok(date) => Ok(ParsedDate::Date(date)),
                Err(_) => Err(
                    Error::new(ErrorKind::InvalidOperation, "invalid date or timestamp")
                        .with_source(err),
                ),
            },
        },
    }
}

// Parse an `OffsetDateTime` from a floating point number.
fn parse_datetime_f64(value: f64) -> Result<ParsedDate, Error> {
    OffsetDateTime::from_unix_timestamp_nanos((value * 1e9) as i128)
        .map(ParsedDate::DateTime)
        .map_err(|_| Error::new(ErrorKind::InvalidOperation, "date out of range"))
}

// Parse an `OffsetDateTime` from a sequence of integers.
fn parse_datetime_seq(value: Value) -> Result<ParsedDate, Error> {
    let mut items = Vec::new();
    for item in value.try_iter()? {
        items.push(i64::try_from(item)?);
    }

    let len = items.len();
    let seq = SeqDeserializer::new(items.into_iter());

    if len == 2 {
        Ok(ParsedDate::Date(
            Date::deserialize(seq).map_err(serde_error)?,
        ))
    } else if len == 6 {
        Ok(ParsedDate::DateTime(
            PrimitiveDateTime::deserialize(seq)
                .map_err(serde_error)?
                .assume_utc(),
        ))
    } else {
        Ok(ParsedDate::DateTime(
            OffsetDateTime::deserialize(seq).map_err(serde_error)?,
        ))
    }
}

fn serde_error(err: serde::de::value::Error) -> Error {
    Error::new(ErrorKind::InvalidOperation, "Not a valid date or time").with_source(err)
}

// Parse a `OffsetDateTime` from a value.
fn parse_datetime(value: Value) -> Result<ParsedDate, Error> {
    if let Some(str) = value.as_str() {
        parse_datetime_string(str)
    } else if let Ok(v) = f64::try_from(value.clone()) {
        parse_datetime_f64(v)
    } else if value.kind() == ValueKind::Seq {
        parse_datetime_seq(value)
    } else {
        Err(Error::new(
            minijinja::ErrorKind::InvalidOperation,
            "value is not a datetime",
        ))
    }
}

const ISO_FORMAT: &[FormatItem<'static>] = format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
);

const ISO_DATE_FORMAT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");

/// Filter to format a datetime.
///
/// This can be used as: `{{ some.value | datetime }}`. By default, an `OffsetDateTime` will be
/// parsed from the value and rendered in an ISO format, which is useful with elements like
/// `<parcel-datetime>`.
///
/// An optional `format` argument can be used to override the formatting of the `OffsetDateTime`.
/// The format should match the format descriptor of the `time` crate, e.g. `[hour]:[minute]`.
pub(super) fn filter_datetime(value: Value, kwargs: Kwargs) -> Result<String, Error> {
    let ParsedDate::DateTime(datetime) = parse_datetime(value)? else {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "expected a datetime",
        ));
    };

    match kwargs.get::<Option<&str>>("format")? {
        None => datetime.format(ISO_FORMAT),
        Some(format) => {
            let format = time::format_description::parse(format).map_err(|err| {
                Error::new(ErrorKind::InvalidOperation, "invalid datetime format").with_source(err)
            })?;

            datetime.format(&format)
        }
    }
    .map_err(|err| {
        Error::new(ErrorKind::InvalidOperation, "failed to format datetime").with_source(err)
    })
}

/// Filter to format a date.
///
/// This can be used as: `{{ some.value | date }}`. By default, a `Date` will be parsed from the
/// value and rendered in an ISO format, which is useful with elements like `<parcel-datetime>`.
///
/// An optional `format` argument can be used to override the formatting of the `Date`.
/// The format should match the format descriptor of the `time` crate, e.g. `[hour]:[minute]`.
///
/// An additional optional `end` argument can be used to specify the end of the day. This is useful
/// for dates that need to signify a whole day, e.g. `{{ some.value | date(end=true) }}`.
pub(super) fn filter_date(value: Value, kwargs: Kwargs) -> Result<String, Error> {
    let ParsedDate::Date(date) = parse_datetime(value)? else {
        return Err(Error::new(ErrorKind::InvalidOperation, "expected a date"));
    };

    match kwargs.get::<Option<&str>>("format")? {
        None => date.format(ISO_DATE_FORMAT),
        Some(format) => {
            let format = time::format_description::parse(format).map_err(|err| {
                Error::new(ErrorKind::InvalidOperation, "invalid date format").with_source(err)
            })?;

            date.format(&format)
        }
    }
    .map_err(|err| {
        tracing::error!(error = ?err, date = ?date, "Failed to format date");
        Error::new(ErrorKind::InvalidOperation, "failed to format date").with_source(err)
    })
}

/// Filter to format a datetime as a human-readable string.
///
/// This can be used as: `{{ some.value | datetime_offset }}`. The value will be parsed as an
/// `OffsetDateTime` and the difference between the current time and the parsed time will be
/// displayed as a human-readable string.
pub(super) fn filter_datetime_offset(value: Value) -> Result<String, Error> {
    let datetime = parse_datetime(value)?;

    Ok(match datetime {
        ParsedDate::Date(date) => {
            let today = OffsetDateTime::now_utc().date();
            if date == today {
                "today".to_string()
            } else {
                time_humanize::HumanTime::from((date - today).whole_seconds()).to_string()
            }
        }

        ParsedDate::DateTime(datetime) => {
            time_humanize::HumanTime::from((datetime - OffsetDateTime::now_utc()).whole_seconds())
                .to_string()
        }
    })
}

fn filter_substr(value: String, kwargs: Kwargs) -> Result<String, Error> {
    let start = kwargs.get::<Option<usize>>("start")?.unwrap_or(0);
    let len = kwargs.get::<Option<usize>>("len")?;
    let end = kwargs.get::<Option<usize>>("end")?;

    if len.is_some() && end.is_some() {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "substr: 'len' and 'end' cannot be used together",
        ));
    }

    let end = if let Some(len) = len {
        start + len
    } else if let Some(end) = end {
        end
    } else {
        value.len()
    };

    Ok(value.chars().skip(start).take(end - start).collect())
}

fn filter_filesizeformat(value: usize, kwargs: Kwargs) -> Result<String, Error> {
    let binary = (kwargs.get::<Option<bool>>("binary")?).unwrap_or(false);

    let format = if binary {
        humansize::BINARY
    } else {
        humansize::WINDOWS
    };

    Ok(humansize::format_size(value, format))
}

fn test_past(value: Value) -> Result<bool, Error> {
    Ok(match parse_datetime(value)? {
        ParsedDate::Date(date) => date < OffsetDateTime::now_utc().date(),
        ParsedDate::DateTime(datetime) => datetime < OffsetDateTime::now_utc(),
    })
}

fn test_future(value: Value) -> Result<bool, Error> {
    Ok(match parse_datetime(value)? {
        ParsedDate::Date(date) => date > OffsetDateTime::now_utc().date(),
        ParsedDate::DateTime(datetime) => datetime > OffsetDateTime::now_utc(),
    })
}

pub(super) fn add_to_environment(environment: &mut Environment) {
    environment.add_filter("datetime", filter_datetime);
    environment.add_filter("datetime_offset", filter_datetime_offset);
    environment.add_filter("date", filter_date);
    environment.add_filter("substr", filter_substr);
    environment.add_filter("filesizeformat", filter_filesizeformat);
    environment.add_test("past", test_past);
    environment.add_test("future", test_future);
}
