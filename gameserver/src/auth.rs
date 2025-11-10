use lazy_static::lazy_static;
use packets::client::ParseError;
use rand::{RngCore, SeedableRng, rngs::StdRng};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};

lazy_static! {
    static ref SECURE_RNG: Mutex<StdRng> = Mutex::new(StdRng::from_os_rng());
}

pub async fn handle_auth(mut stream: TcpStream) -> Result<(), ParseError> {
    let server_seed = SECURE_RNG.lock().await.next_u32();
    let send_packet = packets::server::SMSG_AUTH_CHALLENGE { server_seed };
    stream
        .write_all(&send_packet.to_bytes(None))
        .await
        .map_err(ParseError::Io)?;

    let packet = packets::client::read_specific_packet::<_, packets::client::CMSG_AUTH_SESSION>(
        &mut stream,
        None,
    )
    .await?;

    todo!()
}
