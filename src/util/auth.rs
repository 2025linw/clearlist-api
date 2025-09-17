use std::sync::{Arc, LazyLock};

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum_jwt_auth::{JwtDecoderState, LocalDecoder};
use chrono::Utc;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::ErrorKind,
};
use password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
use uuid::Uuid;

use crate::{
    error::{Error, LOGIN_FAILED, Result},
    models::jwt::Claim,
};

static VALIDATION: LazyLock<Validation> = LazyLock::new(|| {
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_required_spec_claims(&["iss", "aud", "sub", "exp"]);
    validation.set_issuer(&["todo-app-auth"]);
    validation.set_audience(&["todo-app-user"]);

    validation
});

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);

    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Internal(e.to_string()))?
        .to_string())
}

pub fn verify_password(hash: &str, password: &str) -> Result<()> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| Error::Internal(e.to_string()))?;

    match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => (),
        Err(password_hash::Error::Password) => {
            return Err(Error::UserRequest(LOGIN_FAILED.to_string()));
        }
        Err(e) => {
            return Err(Error::Internal(e.to_string()));
        }
    }

    Ok(())
}

pub fn create_decoder(decode_key: &DecodingKey) -> Result<JwtDecoderState<Claim>> {
    let decoder = match LocalDecoder::builder()
        .keys(vec![decode_key.to_owned()])
        .validation(VALIDATION.to_owned())
        .build()
    {
        Ok(d) => d,
        Err(e) => {
            return Err(Error::Internal(e.to_string()));
        }
    };

    Ok(JwtDecoderState {
        decoder: Arc::new(decoder),
    })
}

pub fn create_jwt(encode_key: &EncodingKey, user_id: Uuid, exp: u64) -> Result<String> {
    let header = Header::new(Algorithm::EdDSA);

    let claims = Claim::new(user_id, Utc::now().timestamp() as u64 + exp);

    encode::<Claim>(&header, &claims, encode_key).map_err(|e| Error::Internal(e.to_string()))
}

pub fn verify_jwt_and_get_id(decode_key: &DecodingKey, token: &str) -> Result<Uuid> {
    // TODO: differentiate between auth and refresh token
    let claim = match decode::<Claim>(token, decode_key, &VALIDATION) {
        Ok(t) => t,
        Err(e) => match e.kind() {
            ErrorKind::MissingRequiredClaim(s) => {
                return Err(Error::UserAuth(format!("invalid JWT: {s}")));
            }
            ErrorKind::ExpiredSignature => {
                return Err(Error::UserAuth("expired JWT".to_string()));
            }
            ErrorKind::InvalidIssuer => {
                return Err(Error::UserAuth("invalid issuer for JWT".to_string()));
            }
            ErrorKind::InvalidAudience => {
                return Err(Error::UserAuth("invalid audience for JWT".to_string()));
            }
            e => return Err(Error::Internal(format!("{e:?}"))),
        },
    }
    .claims;

    Ok(claim.sub)
}
