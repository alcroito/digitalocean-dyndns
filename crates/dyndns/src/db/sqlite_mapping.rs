pub use diesel::sql_types::*;
// Override Integer with with BigInt for sqlite for all integer types in the DB schema.
// See https://github.com/diesel-rs/diesel/issues/852
pub type Integer = BigInt;
