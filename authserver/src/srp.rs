use lazy_static::lazy_static;
use num_bigint::BigInt;
use sha1::{Digest, Sha1};

pub const LARGE_SAFE_PRIME_LITTLE_ENDIAN: [u8; 32] = [
    0xb7, 0x9b, 0x3e, 0x2a, 0x87, 0x82, 0x3c, 0xab, 0x8f, 0x5e, 0xbf, 0xbf, 0x8e, 0xb1, 0x01, 0x08,
    0x53, 0x50, 0x06, 0x29, 0x8b, 0x5b, 0xad, 0xbd, 0x5b, 0x53, 0xe1, 0x89, 0x5e, 0x64, 0x4b, 0x89,
];

pub const GENERATOR: u8 = 7;
pub const K_VALUE: u8 = 3;

lazy_static! {
    pub static ref LARGE_SAFE_PRIME: BigInt =
        BigInt::from_bytes_le(num_bigint::Sign::Plus, &LARGE_SAFE_PRIME_LITTLE_ENDIAN);
    pub static ref GENERATOR_BIGINT: BigInt = BigInt::from(GENERATOR);
    pub static ref K_VALUE_BIGINT: BigInt = BigInt::from(K_VALUE);
    pub static ref PRECALCULATED_XOR_HASH: [u8; 20] = Sha1::new()
        .chain_update([GENERATOR])
        .finalize()
        .into_iter()
        .zip(
            Sha1::new()
                .chain_update(LARGE_SAFE_PRIME_LITTLE_ENDIAN)
                .finalize()
                .into_iter()
        )
        .map(|(g, n)| g ^ n)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
}

pub fn calculate_x(username: &str, password: &str, salt: [u8; 32]) -> [u8; 20] {
    let inner_hash = Sha1::new()
        .chain_update(username)
        .chain_update(":")
        .chain_update(password)
        .finalize();

    Sha1::new()
        .chain_update(salt)
        .chain_update(inner_hash)
        .finalize()
        .into()
}

pub fn calculate_password_verifier(username: &str, password: &str, salt: [u8; 32]) -> [u8; 32] {
    let x: [u8; 20] = calculate_x(username, password, salt);
    let x_bigint = BigInt::from_bytes_le(num_bigint::Sign::Plus, &x);
    let mut password_verifier_vec = GENERATOR_BIGINT
        .modpow(&x_bigint, &LARGE_SAFE_PRIME)
        .to_bytes_le()
        .1;
    password_verifier_vec.resize(32, 0);
    password_verifier_vec.try_into().unwrap()
}

pub fn calculate_server_public_key(
    password_verifier: &BigInt,
    server_private_key: &BigInt,
) -> [u8; 32] {
    let mut vec = ((K_VALUE_BIGINT.clone() * password_verifier
        + (GENERATOR_BIGINT.modpow(server_private_key, &LARGE_SAFE_PRIME)))
        % LARGE_SAFE_PRIME.clone())
    .to_bytes_le()
    .1;
    vec.resize(32, 0);
    vec.try_into().unwrap()
}

pub fn calculate_u(client_public_key: [u8; 32], server_public_key: [u8; 32]) -> [u8; 20] {
    Sha1::new()
        .chain_update(client_public_key)
        .chain_update(server_public_key)
        .finalize()
        .into()
}

pub fn calculate_s_key(
    client_public_key: &BigInt,
    password_verifier: &BigInt,
    u: &BigInt,
    server_private_key: &BigInt,
) -> [u8; 32] {
    let s_key = (client_public_key * password_verifier.modpow(u, &LARGE_SAFE_PRIME))
        .modpow(server_private_key, &LARGE_SAFE_PRIME);
    let mut vec = s_key.to_bytes_le().1;
    vec.resize(32, 0);
    vec.try_into().unwrap()
}

pub fn sha_interleave(s_key: [u8; 32]) -> [u8; 40] {
    let mut v_even = Vec::with_capacity(16);
    let mut v_odd = Vec::with_capacity(16);
    s_key
        .into_iter()
        .array_chunks::<2>()
        .skip_while(|v| v[0] == 0)
        .for_each(|[even, odd]| {
            v_even.push(even);
            v_odd.push(odd);
        });

    let g: [u8; 20] = Sha1::new().chain_update(v_even).finalize().into();
    let h: [u8; 20] = Sha1::new().chain_update(v_odd).finalize().into();
    g.into_iter()
        .zip(h)
        .flat_map(|v| [v.0, v.1])
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

pub fn calculate_session_key(
    client_public_key: [u8; 32],
    server_public_key: [u8; 32],
    password_verifier: &BigInt,
    server_private_key: &BigInt,
) -> [u8; 40] {
    let u = calculate_u(client_public_key, server_public_key);
    let s_key = calculate_s_key(
        &BigInt::from_bytes_le(num_bigint::Sign::Plus, &client_public_key),
        password_verifier,
        &BigInt::from_bytes_le(num_bigint::Sign::Plus, &u),
        server_private_key,
    );

    sha_interleave(s_key)
}

pub fn calculate_client_proof(
    username: &str,
    salt: [u8; 32],
    client_public_key: [u8; 32],
    server_public_key: [u8; 32],
    session_key: [u8; 40],
) -> [u8; 20] {
    let hashed_username = Sha1::new().chain_update(username).finalize();
    Sha1::new()
        .chain_update(*PRECALCULATED_XOR_HASH)
        .chain_update(hashed_username)
        .chain_update(salt)
        .chain_update(client_public_key)
        .chain_update(server_public_key)
        .chain_update(session_key)
        .finalize()
        .into()
}

pub fn calculate_server_proof(
    client_public_key: [u8; 32],
    client_proof: [u8; 20],
    session_key: [u8; 40],
) -> [u8; 20] {
    Sha1::new()
        .chain_update(client_public_key)
        .chain_update(client_proof)
        .chain_update(session_key)
        .finalize()
        .into()
}

#[cfg(test)]
mod tests {
    use crate::srp::*;
    use itertools::Itertools;

    #[test]
    fn calculate_x_salt_values() {
        const USERNAME: &str = "USERNAME123";
        const PASSWORD: &str = "PASSWORD123";

        let f = include_str!("../tests/calculate_x_salt_values.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (salt, expected_x) = line.split(" ").collect_tuple().unwrap();
            let mut salt = hex::decode(salt).unwrap();
            salt.reverse();
            let mut expected_x = hex::decode(expected_x).unwrap();
            expected_x.reverse();

            assert_eq!(
                calculate_x(USERNAME, PASSWORD, salt.try_into().unwrap()),
                TryInto::<[u8; 20]>::try_into(expected_x).unwrap()
            );
        }
    }

    #[test]
    fn calculate_x_values() {
        const SALT: &str = "CAC94AF32D817BA64B13F18FDEDEF92AD4ED7EF7AB0E19E9F2AE13C828AEAF57";

        let mut salt = hex::decode(SALT).unwrap();
        salt.reverse();
        let salt: [u8; 32] = salt.try_into().unwrap();

        let f = include_str!("../tests/calculate_x_values.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (username, password, expected_x) = line.split(" ").collect_tuple().unwrap();
            let mut expected_x = hex::decode(expected_x).unwrap();
            expected_x.reverse();

            assert_eq!(
                calculate_x(username, password, salt),
                TryInto::<[u8; 20]>::try_into(expected_x).unwrap()
            );
        }
    }

    #[test]
    fn calculate_b_values() {
        let f = include_str!("../tests/calculate_B_values.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (password_verifier, server_private_key, expected) =
                line.split(" ").collect_tuple().unwrap();
            let password_verifier = hex::decode(password_verifier).unwrap();
            let password_verifier =
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &password_verifier);
            let server_private_key = hex::decode(server_private_key).unwrap();
            let server_private_key =
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &server_private_key);

            let mut expected = hex::decode(expected).unwrap();
            expected.reverse();
            assert_eq!(
                calculate_server_public_key(&password_verifier, &server_private_key),
                TryInto::<[u8; 32]>::try_into(expected).unwrap()
            );
        }
    }

    #[test]
    fn calculate_s_values() {
        let f = include_str!("../tests/calculate_S_values.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (client_public_key, password_verifier, u, server_private_key, expected) =
                line.split(" ").collect_tuple().unwrap();
            let client_public_key = hex::decode(client_public_key).unwrap();
            let client_public_key =
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &client_public_key);
            let password_verifier = hex::decode(password_verifier).unwrap();
            let password_verifier =
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &password_verifier);
            let u = hex::decode(u).unwrap();
            let u = BigInt::from_bytes_be(num_bigint::Sign::Plus, &u);
            let server_private_key = hex::decode(server_private_key).unwrap();
            let server_private_key =
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &server_private_key);

            let mut expected = hex::decode(expected).unwrap();
            expected.reverse();
            assert_eq!(
                calculate_s_key(
                    &client_public_key,
                    &password_verifier,
                    &u,
                    &server_private_key
                ),
                TryInto::<[u8; 32]>::try_into(expected).unwrap()
            );
        }
    }

    #[test]
    fn calculate_u_values() {
        let f = include_str!("../tests/calculate_u_values.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (client_public_key, server_public_key, expected) =
                line.split(" ").collect_tuple().unwrap();
            let mut client_public_key = hex::decode(client_public_key).unwrap();
            client_public_key.reverse();
            let mut server_public_key = hex::decode(server_public_key).unwrap();
            server_public_key.reverse();

            let mut expected = hex::decode(expected).unwrap();
            expected.reverse();
            assert_eq!(
                calculate_u(
                    client_public_key.try_into().unwrap(),
                    server_public_key.try_into().unwrap()
                ),
                TryInto::<[u8; 20]>::try_into(expected).unwrap()
            );
        }
    }

    #[test]
    fn calculate_interleaved_values() {
        let f = include_str!("../tests/calculate_interleaved_values.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (s_key, expected) = line.split(" ").collect_tuple().unwrap();
            assert_eq!(
                sha_interleave(hex::decode(s_key).unwrap().try_into().unwrap()),
                TryInto::<[u8; 40]>::try_into(hex::decode(expected).unwrap()).unwrap()
            );
        }
    }

    #[test]
    fn calculate_m2_values() {
        let f = include_str!("../tests/calculate_M2_values.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (client_public_key, client_proof, session_key, expected) =
                line.split(" ").collect_tuple().unwrap();
            let mut client_public_key = hex::decode(client_public_key).unwrap();
            client_public_key.reverse();
            let mut client_proof = hex::decode(client_proof).unwrap();
            client_proof.reverse();

            let mut expected = hex::decode(expected).unwrap();
            expected.reverse();
            assert_eq!(
                calculate_server_proof(
                    client_public_key.try_into().unwrap(),
                    client_proof.try_into().unwrap(),
                    hex::decode(session_key).unwrap().try_into().unwrap()
                ),
                TryInto::<[u8; 20]>::try_into(expected).unwrap()
            );
        }
    }

    #[test]
    fn calculate_m1_values() {
        let f = include_str!("../tests/calculate_M1_values.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (username, session_key, client_public_key, server_public_key, salt, expected) =
                line.split(" ").collect_tuple().unwrap();
            let mut salt = hex::decode(salt).unwrap();
            salt.reverse();
            let mut client_public_key = hex::decode(client_public_key).unwrap();
            client_public_key.reverse();
            let mut server_public_key = hex::decode(server_public_key).unwrap();
            server_public_key.reverse();

            let mut expected = hex::decode(expected).unwrap();
            expected.reverse();
            assert_eq!(
                calculate_client_proof(
                    username,
                    salt.try_into().unwrap(),
                    client_public_key.try_into().unwrap(),
                    server_public_key.try_into().unwrap(),
                    hex::decode(session_key).unwrap().try_into().unwrap()
                ),
                TryInto::<[u8; 20]>::try_into(expected).unwrap()
            );
        }
    }
}
