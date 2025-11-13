use byteorder::{ByteOrder, WriteBytesExt};

use crate::server::OrderedWrite;

#[allow(non_camel_case_types)]
#[allow(unused)]
pub enum AccountResult {
    RESPONSE_SUCCESS,
    RESPONSE_FAILURE,
    RESPONSE_CANCELLED,
    RESPONSE_DISCONNECTED,
    RESPONSE_FAILED_TO_CONNECT,
    RESPONSE_CONNECTED,
    RESPONSE_VERSION_MISMATCH,
    CSTATUS_CONNECTING,
    CSTATUS_NEGOTIATING_SECURITY,
    CSTATUS_NEGOTIATION_COMPLETE,
    CSTATUS_NEGOTIATION_FAILED,
    CSTATUS_AUTHENTICATING,
    AUTH_OK {
        billing_time: u32,
        billing_flags: u8,
        billing_rested: u32,
    },
    AUTH_FAILED,
    AUTH_REJECT,
    AUTH_BAD_SERVER_PROOF,
    AUTH_UNAVAILABLE,
    AUTH_SYSTEM_ERROR,
    AUTH_BILLING_ERROR,
    AUTH_BILLING_EXPIRED,
    AUTH_VERSION_MISMATCH,
    AUTH_UNKNOWN_ACCOUNT,
    AUTH_INCORRECT_PASSWORD,
    AUTH_SESSION_EXPIRED,
    AUTH_SERVER_SHUTTING_DOWN,
    AUTH_ALREADY_LOGGING_IN,
    AUTH_LOGIN_SERVER_NOT_FOUND,
    AUTH_WAIT_QUEUE {
        queue_position: u32,
    },
    AUTH_BANNED,
    AUTH_ALREADY_ONLINE,
    AUTH_NO_TIME,
    AUTH_DB_BUSY,
    AUTH_SUSPENDED,
    AUTH_PARENTAL_CONTROL,
    REALM_LIST_IN_PROGRESS,
    REALM_LIST_SUCCESS,
    REALM_LIST_FAILED,
    REALM_LIST_INVALID,
    REALM_LIST_REALM_NOT_FOUND,
    ACCOUNT_CREATE_IN_PROGRESS,
    ACCOUNT_CREATE_SUCCESS,
    ACCOUNT_CREATE_FAILED,
    CHAR_LIST_RETRIEVING,
    CHAR_LIST_RETRIEVED,
    CHAR_LIST_FAILED,
    CHAR_CREATE_IN_PROGRESS,
    CHAR_CREATE_SUCCESS,
    CHAR_CREATE_ERROR,
    CHAR_CREATE_FAILED,
    CHAR_CREATE_NAME_IN_USE,
    CHAR_CREATE_DISABLED,
    CHAR_CREATE_PVP_TEAMS_VIOLATION,
    CHAR_CREATE_SERVER_LIMIT,
    CHAR_CREATE_ACCOUNT_LIMIT,
    CHAR_CREATE_SERVER_QUEUE,
    CHAR_CREATE_ONLY_EXISTING,
    CHAR_DELETE_IN_PROGRESS,
    CHAR_DELETE_SUCCESS,
    CHAR_DELETE_FAILED,
    CHAR_DELETE_FAILED_LOCKED_FOR_TRANSFER,
    CHAR_LOGIN_IN_PROGRESS,
    CHAR_LOGIN_SUCCESS,
    CHAR_LOGIN_NO_WORLD,
    CHAR_LOGIN_DUPLICATE_CHARACTER,
    CHAR_LOGIN_NO_INSTANCES,
    CHAR_LOGIN_FAILED,
    CHAR_LOGIN_DISABLED,
    CHAR_LOGIN_NO_CHARACTER,
    CHAR_LOGIN_LOCKED_FOR_TRANSFER,
    CHAR_NAME_NO_NAME,
    CHAR_NAME_TOO_SHORT,
    CHAR_NAME_TOO_LONG,
    CHAR_NAME_ONLY_LETTERS,
    CHAR_NAME_MIXED_LANGUAGES,
    CHAR_NAME_PROFANE,
    CHAR_NAME_RESERVED,
    CHAR_NAME_INVALID_APOSTROPHE,
    CHAR_NAME_MULTIPLE_APOSTROPHES,
    CHAR_NAME_THREE_CONSECUTIVE,
    CHAR_NAME_INVALID_SPACE,
    CHAR_NAME_SUCCESS,
    CHAR_NAME_FAILURE,
}

impl<T: ByteOrder> OrderedWrite<T> for AccountResult {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        match self {
            AccountResult::RESPONSE_SUCCESS => {
                writer.write_u32::<T>(0x00)?;
            }
            AccountResult::RESPONSE_FAILURE => {
                writer.write_u32::<T>(0x01)?;
            }
            AccountResult::RESPONSE_CANCELLED => {
                writer.write_u32::<T>(0x02)?;
            }
            AccountResult::RESPONSE_DISCONNECTED => {
                writer.write_u32::<T>(0x03)?;
            }
            AccountResult::RESPONSE_FAILED_TO_CONNECT => {
                writer.write_u32::<T>(0x04)?;
            }
            AccountResult::RESPONSE_CONNECTED => {
                writer.write_u32::<T>(0x05)?;
            }
            AccountResult::RESPONSE_VERSION_MISMATCH => {
                writer.write_u32::<T>(0x06)?;
            }
            AccountResult::CSTATUS_CONNECTING => {
                writer.write_u32::<T>(0x07)?;
            }
            AccountResult::CSTATUS_NEGOTIATING_SECURITY => {
                writer.write_u32::<T>(0x08)?;
            }
            AccountResult::CSTATUS_NEGOTIATION_COMPLETE => {
                writer.write_u32::<T>(0x09)?;
            }
            AccountResult::CSTATUS_NEGOTIATION_FAILED => {
                writer.write_u32::<T>(0x0A)?;
            }
            AccountResult::CSTATUS_AUTHENTICATING => {
                writer.write_u32::<T>(0x0B)?;
            }
            AccountResult::AUTH_OK {
                billing_time,
                billing_flags,
                billing_rested,
            } => {
                writer.write_u32::<T>(0x0C)?;
                writer.write_u32::<T>(*billing_time)?;
                writer.write_u8(*billing_flags)?;
                writer.write_u32::<T>(*billing_rested)?;
            }
            AccountResult::AUTH_FAILED => {
                writer.write_u32::<T>(0x0D)?;
            }
            AccountResult::AUTH_REJECT => {
                writer.write_u32::<T>(0x0E)?;
            }
            AccountResult::AUTH_BAD_SERVER_PROOF => {
                writer.write_u32::<T>(0x0F)?;
            }
            AccountResult::AUTH_UNAVAILABLE => {
                writer.write_u32::<T>(0x10)?;
            }
            AccountResult::AUTH_SYSTEM_ERROR => {
                writer.write_u32::<T>(0x11)?;
            }
            AccountResult::AUTH_BILLING_ERROR => {
                writer.write_u32::<T>(0x12)?;
            }
            AccountResult::AUTH_BILLING_EXPIRED => {
                writer.write_u32::<T>(0x13)?;
            }
            AccountResult::AUTH_VERSION_MISMATCH => {
                writer.write_u32::<T>(0x14)?;
            }
            AccountResult::AUTH_UNKNOWN_ACCOUNT => {
                writer.write_u32::<T>(0x15)?;
            }
            AccountResult::AUTH_INCORRECT_PASSWORD => {
                writer.write_u32::<T>(0x16)?;
            }
            AccountResult::AUTH_SESSION_EXPIRED => {
                writer.write_u32::<T>(0x17)?;
            }
            AccountResult::AUTH_SERVER_SHUTTING_DOWN => {
                writer.write_u32::<T>(0x18)?;
            }
            AccountResult::AUTH_ALREADY_LOGGING_IN => {
                writer.write_u32::<T>(0x19)?;
            }
            AccountResult::AUTH_LOGIN_SERVER_NOT_FOUND => {
                writer.write_u32::<T>(0x1A)?;
            }
            AccountResult::AUTH_WAIT_QUEUE { queue_position } => {
                writer.write_u32::<T>(0x1B)?;
                writer.write_u8(0)?;
                writer.write_u8(0)?;
                writer.write_u8(0)?;
                writer.write_u8(0)?;
                writer.write_u8(0)?;
                writer.write_u32::<T>(*queue_position)?;
            }
            AccountResult::AUTH_BANNED => {
                writer.write_u32::<T>(0x1C)?;
            }
            AccountResult::AUTH_ALREADY_ONLINE => {
                writer.write_u32::<T>(0x1D)?;
            }
            AccountResult::AUTH_NO_TIME => {
                writer.write_u32::<T>(0x1E)?;
            }
            AccountResult::AUTH_DB_BUSY => {
                writer.write_u32::<T>(0x1F)?;
            }
            AccountResult::AUTH_SUSPENDED => {
                writer.write_u32::<T>(0x20)?;
            }
            AccountResult::AUTH_PARENTAL_CONTROL => {
                writer.write_u32::<T>(0x21)?;
            }
            AccountResult::REALM_LIST_IN_PROGRESS => {
                writer.write_u32::<T>(0x22)?;
            }
            AccountResult::REALM_LIST_SUCCESS => {
                writer.write_u32::<T>(0x23)?;
            }
            AccountResult::REALM_LIST_FAILED => {
                writer.write_u32::<T>(0x24)?;
            }
            AccountResult::REALM_LIST_INVALID => {
                writer.write_u32::<T>(0x25)?;
            }
            AccountResult::REALM_LIST_REALM_NOT_FOUND => {
                writer.write_u32::<T>(0x26)?;
            }
            AccountResult::ACCOUNT_CREATE_IN_PROGRESS => {
                writer.write_u32::<T>(0x27)?;
            }
            AccountResult::ACCOUNT_CREATE_SUCCESS => {
                writer.write_u32::<T>(0x28)?;
            }
            AccountResult::ACCOUNT_CREATE_FAILED => {
                writer.write_u32::<T>(0x29)?;
            }
            AccountResult::CHAR_LIST_RETRIEVING => {
                writer.write_u32::<T>(0x2A)?;
            }
            AccountResult::CHAR_LIST_RETRIEVED => {
                writer.write_u32::<T>(0x2B)?;
            }
            AccountResult::CHAR_LIST_FAILED => {
                writer.write_u32::<T>(0x2C)?;
            }
            AccountResult::CHAR_CREATE_IN_PROGRESS => {
                writer.write_u32::<T>(0x2D)?;
            }
            AccountResult::CHAR_CREATE_SUCCESS => {
                writer.write_u32::<T>(0x2E)?;
            }
            AccountResult::CHAR_CREATE_ERROR => {
                writer.write_u32::<T>(0x2F)?;
            }
            AccountResult::CHAR_CREATE_FAILED => {
                writer.write_u32::<T>(0x30)?;
            }
            AccountResult::CHAR_CREATE_NAME_IN_USE => {
                writer.write_u32::<T>(0x31)?;
            }
            AccountResult::CHAR_CREATE_DISABLED => {
                writer.write_u32::<T>(0x32)?;
            }
            AccountResult::CHAR_CREATE_PVP_TEAMS_VIOLATION => {
                writer.write_u32::<T>(0x33)?;
            }
            AccountResult::CHAR_CREATE_SERVER_LIMIT => {
                writer.write_u32::<T>(0x34)?;
            }
            AccountResult::CHAR_CREATE_ACCOUNT_LIMIT => {
                writer.write_u32::<T>(0x35)?;
            }
            AccountResult::CHAR_CREATE_SERVER_QUEUE => {
                writer.write_u32::<T>(0x36)?;
            }
            AccountResult::CHAR_CREATE_ONLY_EXISTING => {
                writer.write_u32::<T>(0x37)?;
            }
            AccountResult::CHAR_DELETE_IN_PROGRESS => {
                writer.write_u32::<T>(0x38)?;
            }
            AccountResult::CHAR_DELETE_SUCCESS => {
                writer.write_u32::<T>(0x39)?;
            }
            AccountResult::CHAR_DELETE_FAILED => {
                writer.write_u32::<T>(0x3A)?;
            }
            AccountResult::CHAR_DELETE_FAILED_LOCKED_FOR_TRANSFER => {
                writer.write_u32::<T>(0x3B)?;
            }
            AccountResult::CHAR_LOGIN_IN_PROGRESS => {
                writer.write_u32::<T>(0x3C)?;
            }
            AccountResult::CHAR_LOGIN_SUCCESS => {
                writer.write_u32::<T>(0x3D)?;
            }
            AccountResult::CHAR_LOGIN_NO_WORLD => {
                writer.write_u32::<T>(0x3E)?;
            }
            AccountResult::CHAR_LOGIN_DUPLICATE_CHARACTER => {
                writer.write_u32::<T>(0x3F)?;
            }
            AccountResult::CHAR_LOGIN_NO_INSTANCES => {
                writer.write_u32::<T>(0x40)?;
            }
            AccountResult::CHAR_LOGIN_FAILED => {
                writer.write_u32::<T>(0x41)?;
            }
            AccountResult::CHAR_LOGIN_DISABLED => {
                writer.write_u32::<T>(0x42)?;
            }
            AccountResult::CHAR_LOGIN_NO_CHARACTER => {
                writer.write_u32::<T>(0x43)?;
            }
            AccountResult::CHAR_LOGIN_LOCKED_FOR_TRANSFER => {
                writer.write_u32::<T>(0x44)?;
            }
            AccountResult::CHAR_NAME_NO_NAME => {
                writer.write_u32::<T>(0x45)?;
            }
            AccountResult::CHAR_NAME_TOO_SHORT => {
                writer.write_u32::<T>(0x46)?;
            }
            AccountResult::CHAR_NAME_TOO_LONG => {
                writer.write_u32::<T>(0x47)?;
            }
            AccountResult::CHAR_NAME_ONLY_LETTERS => {
                writer.write_u32::<T>(0x48)?;
            }
            AccountResult::CHAR_NAME_MIXED_LANGUAGES => {
                writer.write_u32::<T>(0x49)?;
            }
            AccountResult::CHAR_NAME_PROFANE => {
                writer.write_u32::<T>(0x4A)?;
            }
            AccountResult::CHAR_NAME_RESERVED => {
                writer.write_u32::<T>(0x4B)?;
            }
            AccountResult::CHAR_NAME_INVALID_APOSTROPHE => {
                writer.write_u32::<T>(0x4C)?;
            }
            AccountResult::CHAR_NAME_MULTIPLE_APOSTROPHES => {
                writer.write_u32::<T>(0x4D)?;
            }
            AccountResult::CHAR_NAME_THREE_CONSECUTIVE => {
                writer.write_u32::<T>(0x4E)?;
            }
            AccountResult::CHAR_NAME_INVALID_SPACE => {
                writer.write_u32::<T>(0x4F)?;
            }
            AccountResult::CHAR_NAME_SUCCESS => {
                writer.write_u32::<T>(0x50)?;
            }
            AccountResult::CHAR_NAME_FAILURE => {
                writer.write_u32::<T>(0x51)?;
            }
        }

        Ok(())
    }
}
