use serde::{Deserialize, Serialize};

/// The response code for a command.
///
/// The HTTP Status in a response may
/// only tell if a command could be error free received, but not if it was in any way valid
/// and could be processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ResponseCode {
    pub code: u32,
    pub message: &'static str,
}

impl ResponseCode {
    /// This is a code that means "Ok".
    ///
    /// Use this function instead of comparing the code to "1000",
    /// it can be expanded if it becomes clearer (reverse-engineered) what other codes mean "Ok".
    ///
    /// That could be context-dependent.
    pub fn is_ok(&self) -> bool {
        self.code == OK.code
    }

    /// This is a code which means "Error".
    pub fn is_error(&self) -> bool {
        !self.is_ok()
    }
}

// Errors codes from https://xled-docs.readthedocs.io/en/latest/rest_api.html#http-responses.

/// The OK response code.
pub const OK: ResponseCode = ResponseCode {
    code: 1000,
    message: "Ok",
};
/// An error response code.
pub const ERROR: ResponseCode = ResponseCode {
    code: 1001,
    message: "Error",
};
/// An error response code.
pub const ERROR_INVALID_ARGUMENT: ResponseCode = ResponseCode {
    code: 1101,
    message: "Invalid argument value",
};
/// An error response code.
pub const ERROR2: ResponseCode = ResponseCode {
    code: 1102,
    message: "Error",
};
/// An error response code.
pub const ERROR_VALUE_WRONG_MISSING_KEY: ResponseCode = ResponseCode {
    code: 1103,
    message: "Error - value too long? Or missing required object key?",
};
/// An error response code.
pub const ERROR_MALFORMED_JSON_INPUT: ResponseCode = ResponseCode {
    code: 1104,
    message: "Error - malformed JSON on input?",
};
/// An error response code.
pub const ERROR_INVALID_ARGUMENT_KEY: ResponseCode = ResponseCode {
    code: 1105,
    message: "Invalid argument key",
};
/// An OK response code?
pub const OK2: ResponseCode = ResponseCode {
    code: 1107,
    message: "OK?",
};
/// An OK response code?
pub const OK3: ResponseCode = ResponseCode {
    code: 1108,
    message: "OK?",
};
/// An error response code.
pub const FIRMWARE_UPGRADE_ERROR: ResponseCode = ResponseCode {
    code: 1205,
    message: "Error with firmware upgrade - SHA1SUM does not match",
};

/// Trait for response codes.
pub trait ResponseCodeTrait {
    /// Get the response code.
    /// # Returns
    /// The response code.
    fn response_code(&self) -> ResponseCode;

    fn map_response_code(code: u32) -> ResponseCode {
        match code {
            1000 => OK,
            1001 => ERROR,
            1101 => ERROR_INVALID_ARGUMENT,
            1102 => ERROR2,
            1103 => ERROR_VALUE_WRONG_MISSING_KEY,
            1104 => ERROR_MALFORMED_JSON_INPUT,
            1105 => ERROR_INVALID_ARGUMENT_KEY,
            1107 => OK2,
            1108 => OK3,
            1205 => FIRMWARE_UPGRADE_ERROR,
            _ => ERROR,
        }
    }
}
