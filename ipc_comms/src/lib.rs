pub mod realm_types;

use std::net::SocketAddr;

use bincode::{Decode, Encode};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::realm_types::{RealmCategory, RealmType};

// Game server -> Auth server
#[derive(Decode, Encode)]
pub enum AuthServerIpcMessage {
    RealmInfo {
        realm_id: u8,
        realm_type: RealmType,
        flags: u8,
        realm_name: String,
        address_port: SocketAddr,
        population: f32,
        realm_category: RealmCategory,
    },
    PlayerSessionKeyRequest {
        account_name: String, // Game server can only access account name at first
    },
    PlayerNumCharactersResponse {
        account_id: u32,
        num_characters: u8,
    },
    // UpdateNumCharactersCache?
    GameServerClosed,
}

impl AuthServerIpcMessage {
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self, IpcError> {
        let length = reader.read_u32().await.map_err(IpcError::Io)?;

        let mut buf = vec![0; length as usize];
        reader.read_exact(&mut buf).await.map_err(IpcError::Io)?;

        let r = bincode::decode_from_slice(&buf, bincode::config::standard())
            .map_err(IpcError::Decode)?;

        // num written should equal num read, if not - we're somehow not in the start of the message
        assert_eq!(r.1, length as usize);

        Ok(r.0)
    }

    pub async fn write<W: AsyncWrite + Unpin>(self, writer: &mut W) -> Result<(), IpcError> {
        let buf =
            bincode::encode_to_vec(self, bincode::config::standard()).map_err(IpcError::Encode)?;

        // This should never be the case
        assert!(buf.len() <= u32::MAX as usize);
        writer
            .write_u32(buf.len() as u32)
            .await
            .map_err(IpcError::Io)?;

        writer.write_all(&buf).await.map_err(IpcError::Io)?;

        Ok(())
    }
}

#[derive(Decode, Encode)]
pub enum SessionKeyResponse {
    Authenticated {
        account_id: u32, // lets the server know the account id of the player
        session_key: [u8; 40],
    },
    Unauthenticated,
}

// Auth server -> Game server
#[derive(Decode, Encode)]
pub enum GameServerIpcMessage {
    PlayerSessionKeyResponse {
        account_name: String,
        session_key: SessionKeyResponse, // if None -> user is not authenticated
    },
    PlayerNumCharactersRequest {
        account_id: u32,
    },
    AuthServerError(AuthServerError),
    AuthServerClosed,
}

impl GameServerIpcMessage {
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self, IpcError> {
        let length = reader.read_u32().await.map_err(IpcError::Io)?;

        let mut buf = vec![0; length as usize];
        reader.read_exact(&mut buf).await.map_err(IpcError::Io)?;

        let r = bincode::decode_from_slice(&buf, bincode::config::standard())
            .map_err(IpcError::Decode)?;

        // num written should equal num read, if not - we're somehow not in the start of the message
        assert_eq!(r.1, length as usize);

        Ok(r.0)
    }

    pub async fn write<W: AsyncWrite + Unpin>(self, writer: &mut W) -> Result<(), IpcError> {
        let buf =
            bincode::encode_to_vec(self, bincode::config::standard()).map_err(IpcError::Encode)?;

        // This should never be the case
        assert!(buf.len() <= u32::MAX as usize);
        writer
            .write_u32(buf.len() as u32)
            .await
            .map_err(IpcError::Io)?;

        writer.write_all(&buf).await.map_err(IpcError::Io)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum IpcError {
    Io(std::io::Error),
    Decode(bincode::error::DecodeError),
    Encode(bincode::error::EncodeError),
}

#[derive(Debug, Decode, Encode)]
pub enum AuthServerError {
    DuplicateRealmInfo { previous_id: u8, new_id: u8 },
}
