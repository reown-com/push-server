// mod env;
// mod providers;
// mod store; // Comment this out for now
// `context` only supports the functional tests, so gate it the same way. Without this,
// feature sets that compile the test crate but not `functional` (default, multitenant)
// see the whole module as dead code and `-D warnings` fails clippy.
#[cfg(feature = "functional_tests")]
mod context;
#[cfg(feature = "functional_tests")]
mod functional;
mod unit;

pub type ErrorResult<T> = Result<T, TestError>;

#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error(transparent)]
    Elapsed(#[from] tokio::time::error::Elapsed),

    #[error(transparent)]
    EchoServer(#[from] echo_server::error::Error),
}
