use std::fmt::{Debug, Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    EnvyError(envy::Error),
    TraceError(opentelemetry::trace::TraceError),
    MetricsError(opentelemetry::metrics::MetricsError),
    ProviderNotFound,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EnvyError(err) => write!(f, "{}", err),
            Error::TraceError(err) => Debug::fmt(&err, f),
            Error::MetricsError(err) => Debug::fmt(&err, f),
            Error::ProviderNotFound => write!(f, "push provider not found"),
        }
    }
}

impl From<envy::Error> for Error {
    fn from(err: envy::Error) -> Self {
        Error::EnvyError(err)
    }
}

impl From<opentelemetry::trace::TraceError> for Error {
    fn from(err: opentelemetry::trace::TraceError) -> Self {
        Error::TraceError(err)
    }
}

impl From<opentelemetry::metrics::MetricsError> for Error {
    fn from(err: opentelemetry::metrics::MetricsError) -> Self {
        Error::MetricsError(err)
    }
}

impl std::error::Error for Error {}
