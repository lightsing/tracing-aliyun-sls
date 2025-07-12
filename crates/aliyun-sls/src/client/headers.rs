#![allow(dead_code)]
pub const AUTHORIZATION: &str = "authorization";
pub const CONTENT_LENGTH: &str = "content-length";
pub const CONTENT_TYPE: &str = "content-type";
pub const DATE: &str = "date";
pub const API_VERSION: &str = "0.6.0";
pub const LOG_API_VERSION: &str = "x-log-apiversion";
pub const LOG_SIGNATURE_METHOD: &str = "x-log-signaturemethod";
pub const LOG_BODY_RAW_SIZE: &str = "x-log-bodyrawsize";
pub const LOG_COMPRESS_TYPE: &str = "x-log-compresstype";

pub const CONTENT_MD5: &str = "content-md5";
pub const USER_AGENT_VALUE: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const DEFAULT_CONTENT_TYPE: &str = "application/x-protobuf";
pub const SIGNATURE_METHOD: &str = "hmac-sha1";
