//! Password protection of a project file.
//!
//! A protected `.cre` is still a JSON text, with its own top-level key, so every program reads it as text and knows it at once.
//! The project (the JSON of a plain `.cre`) is encrypted with XChaCha20-Poly1305 under a random 256-bit *data key*, with a new
//! random nonce at every save. The data key is kept in the file wrapped once for every password: the **owner** password (it may
//! do everything) and, when there is one, the **open** password (it may open, look, make G-code, run the job and change the
//! settings of the material, but not edit or copy the drawing). A key is made from a password by Argon2id with a salt of its own;
//! the parameters are kept in the slot, so later versions can raise them and still read older files. The header (the format, the
//! roles and the parameters) is bound to every ciphertext as associated data: a changed header does not open.
//!
//! What the open password forbids is enforced by the programs, not by the cryptography: whoever has it can decrypt the project
//! (as with the owner password of a PDF). Without a password the file cannot be read.
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

/// The top-level key of a protected file; its value is the version of the format.
pub const KEY: &str = "protected_cre";
pub const FORMAT: u32 = 1;
/// The shortest password accepted.
pub const MIN_PASSWORD: usize = 8;

/// What a password allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// Everything: editing, copying, changing or removing the protection.
    Owner,
    /// Opening, looking, G-code, the job and the material; no editing and no copying of the drawing.
    Open,
}

/// The cost of Argon2id: memory in KiB, passes, lanes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kdf {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

/// 64 MiB, 3 passes, 1 lane: about a quarter of a second on a usual computer (OWASP's advice is at least 19 MiB and 2 passes).
pub const KDF: Kdf = Kdf { m_kib: 64 * 1024, t: 3, p: 1 };
/// The least that is accepted from a file, so a changed header cannot make the guessing cheap.
const KDF_MIN: Kdf = Kdf { m_kib: 8 * 1024, t: 1, p: 1 };
/// The most that is accepted from a file (a header could ask for memory that is not there).
const KDF_MAX: Kdf = Kdf { m_kib: 4 * 1024 * 1024, t: 64, p: 16 };

/// The data key wrapped for one password.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub role: Access,
    pub kdf: Kdf,
    pub salt: String,
    pub nonce: String,
    /// The data key encrypted with the key of the password (32 bytes and the 16 of the tag).
    pub key: String,
}

/// The whole protected file.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Sealed {
    protected_cre: u32,
    cipher: String,
    kdf: String,
    slots: Vec<Slot>,
    nonce: String,
    data: String,
}

const CIPHER: &str = "XChaCha20-Poly1305";
const KDF_NAME: &str = "Argon2id";

/// A protection in memory: the data key and the wrapped copies of it, and what the person who opened the file may do. Saving
/// with it needs no password (no key is made again), so it is cheap enough for the automatic saves.
#[derive(Clone)]
pub struct Protection {
    key: Zeroizing<[u8; 32]>,
    slots: Vec<Slot>,
    pub access: Access,
}

impl std::fmt::Debug for Protection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Protection").field("slots", &self.slots.len()).field("access", &self.access).finish_non_exhaustive()
    }
}

/// Why a file did not open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// No slot opens with this password.
    WrongPassword,
    /// The file is damaged or not a protected project.
    Damaged(String),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::WrongPassword => write!(f, "wrong password"),
            OpenError::Damaged(e) => write!(f, "the protected file is damaged: {e}"),
        }
    }
}

fn random<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).expect("the system gives random bytes");
    b
}

fn b64(b: &[u8]) -> String {
    B64.encode(b)
}

fn unb64(s: &str) -> Result<Vec<u8>, OpenError> {
    B64.decode(s).map_err(|e| OpenError::Damaged(e.to_string()))
}

/// The key of a password.
fn derive(password: &str, salt: &[u8], kdf: Kdf) -> Result<Zeroizing<[u8; 32]>, String> {
    let params = argon2::Params::new(kdf.m_kib, kdf.t, kdf.p, Some(32)).map_err(|e| e.to_string())?;
    let a = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    a.hash_password_into(password.as_bytes(), salt, out.as_mut()).map_err(|e| e.to_string())?;
    Ok(out)
}

fn cipher(key: &[u8; 32]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new_from_slice(key).expect("a key of 32 bytes")
}

/// The associated data of a slot: its role and its parameters, so they cannot be changed.
fn slot_aad(role: Access, kdf: Kdf) -> Vec<u8> {
    format!("{KEY}/{FORMAT}/slot/{role:?}/{}/{}/{}", kdf.m_kib, kdf.t, kdf.p).into_bytes()
}

/// The associated data of the project: the header with every slot.
fn data_aad(slots: &[Slot]) -> Vec<u8> {
    let mut s = format!("{KEY}/{FORMAT}/{CIPHER}/{KDF_NAME}");
    for sl in slots {
        s.push_str(&format!("/{:?}:{}:{}:{}:{}:{}:{}", sl.role, sl.kdf.m_kib, sl.kdf.t, sl.kdf.p, sl.salt, sl.nonce, sl.key));
    }
    s.into_bytes()
}

fn wrap(data_key: &[u8; 32], password: &str, role: Access, kdf: Kdf) -> Result<Slot, String> {
    let salt = random::<16>();
    let nonce = random::<24>();
    let kek = derive(password, &salt, kdf)?;
    let key = cipher(&kek)
        .encrypt(&XNonce::try_from(&nonce[..]).expect("24 bytes"), Payload { msg: data_key, aad: &slot_aad(role, kdf) })
        .map_err(|_| "encryption failed".to_string())?;
    Ok(Slot { role, kdf, salt: b64(&salt), nonce: b64(&nonce), key: b64(&key) })
}

fn unwrap(slot: &Slot, password: &str) -> Result<Option<Zeroizing<[u8; 32]>>, OpenError> {
    let k = slot.kdf;
    if k.m_kib < KDF_MIN.m_kib || k.t < KDF_MIN.t || k.p < KDF_MIN.p || k.m_kib > KDF_MAX.m_kib || k.t > KDF_MAX.t || k.p > KDF_MAX.p {
        return Err(OpenError::Damaged("the key parameters are out of range".into()));
    }
    let (salt, nonce, wrapped) = (unb64(&slot.salt)?, unb64(&slot.nonce)?, unb64(&slot.key)?);
    let nonce = XNonce::try_from(&nonce[..]).map_err(|_| OpenError::Damaged("a nonce of the wrong size".into()))?;
    let kek = derive(password, &salt, k).map_err(OpenError::Damaged)?;
    let Ok(mut plain) = cipher(&kek).decrypt(&nonce, Payload { msg: &wrapped, aad: &slot_aad(slot.role, k) }) else { return Ok(None) };
    if plain.len() != 32 {
        plain.zeroize();
        return Err(OpenError::Damaged("a key of the wrong size".into()));
    }
    let mut key = Zeroizing::new([0u8; 32]);
    key.copy_from_slice(&plain);
    plain.zeroize();
    Ok(Some(key))
}

impl Protection {
    /// A new protection: a new data key, wrapped for the owner password and, when given, for the open password. The person who
    /// sets it is the owner. Slow (a key is made for every password): do it once, when the passwords are set.
    pub fn new(owner: &str, open: Option<&str>) -> Result<Protection, String> {
        Self::with_kdf(owner, open, KDF)
    }

    pub fn with_kdf(owner: &str, open: Option<&str>, kdf: Kdf) -> Result<Protection, String> {
        check_passwords(owner, open)?;
        let key = Zeroizing::new(random::<32>());
        let mut slots = vec![wrap(&key, owner, Access::Owner, kdf)?];
        if let Some(open) = open.filter(|o| !o.is_empty()) {
            slots.push(wrap(&key, open, Access::Open, kdf)?);
        }
        Ok(Protection { key, slots, access: Access::Owner })
    }

    /// The owner password given for a project opened with the open password: from now on the owner may do everything (the same
    /// key, so nothing is saved again). Slow (the key of the password is made).
    pub fn unlock_owner(&mut self, password: &str) -> Result<(), OpenError> {
        let Some(slot) = self.slots.iter().find(|s| s.role == Access::Owner) else { return Err(OpenError::Damaged("no owner password".into())) };
        match unwrap(slot, password)? {
            Some(key) if *key == *self.key => {
                self.access = Access::Owner;
                Ok(())
            }
            Some(_) => Err(OpenError::Damaged("the owner password is for another key".into())),
            None => Err(OpenError::WrongPassword),
        }
    }

    /// Whether the file has an open password too (besides the owner's).
    pub fn has_open_password(&self) -> bool {
        self.slots.iter().any(|s| s.role == Access::Open)
    }

    /// Encrypt the JSON of a project into the text of a protected file, with a new nonce.
    pub fn seal(&self, json: &str) -> String {
        let nonce = random::<24>();
        let data = cipher(&self.key)
            .encrypt(&XNonce::try_from(&nonce[..]).expect("24 bytes"), Payload { msg: json.as_bytes(), aad: &data_aad(&self.slots) })
            .expect("encryption of a buffer in memory");
        let s = Sealed {
            protected_cre: FORMAT,
            cipher: CIPHER.into(),
            kdf: KDF_NAME.into(),
            slots: self.slots.clone(),
            nonce: b64(&nonce),
            data: b64(&data),
        };
        serde_json::to_string(&s).expect("a protected file is plain JSON")
    }
}

/// The rules for passwords: the owner password long enough, the open one too when given, and not the same as the owner's.
pub fn check_passwords(owner: &str, open: Option<&str>) -> Result<(), String> {
    if owner.chars().count() < MIN_PASSWORD {
        return Err(format!("the owner password needs at least {MIN_PASSWORD} characters"));
    }
    if let Some(open) = open.filter(|o| !o.is_empty()) {
        if open.chars().count() < MIN_PASSWORD {
            return Err(format!("the open password needs at least {MIN_PASSWORD} characters"));
        }
        if open == owner {
            return Err("the open password must differ from the owner password".into());
        }
    }
    Ok(())
}

/// Is this the text of a protected project? Looks only at the start, so a big plain project is not parsed for it.
pub fn is_sealed(text: &str) -> bool {
    let head: String = text.chars().take(64).filter(|c| !c.is_whitespace()).collect();
    head.starts_with(&format!("{{\"{KEY}\""))
}

/// Open a protected project with a password: the JSON of the project and the protection to save it with again (its `access`
/// says what the password allows). The owner slot is tried first.
pub fn open(text: &str, password: &str) -> Result<(String, Protection), OpenError> {
    let s: Sealed = serde_json::from_str(text).map_err(|e| OpenError::Damaged(e.to_string()))?;
    if s.protected_cre != FORMAT {
        return Err(OpenError::Damaged(format!("format {} is newer than this program knows: update the program", s.protected_cre)));
    }
    if s.cipher != CIPHER || s.kdf != KDF_NAME {
        return Err(OpenError::Damaged(format!("{} / {} is not known", s.cipher, s.kdf)));
    }
    if s.slots.is_empty() || s.slots.len() > 4 || !s.slots.iter().any(|sl| sl.role == Access::Owner) {
        return Err(OpenError::Damaged("the passwords of the file are missing".into()));
    }
    let mut order: Vec<&Slot> = s.slots.iter().collect();
    order.sort_by_key(|sl| sl.role != Access::Owner);
    for slot in order {
        let Some(key) = unwrap(slot, password)? else { continue };
        let nonce = unb64(&s.nonce)?;
        let nonce = XNonce::try_from(&nonce[..]).map_err(|_| OpenError::Damaged("a nonce of the wrong size".into()))?;
        let data = unb64(&s.data)?;
        let plain = cipher(&key)
            .decrypt(&nonce, Payload { msg: &data, aad: &data_aad(&s.slots) })
            .map_err(|_| OpenError::Damaged("the content does not match its seal (changed or broken)".into()))?;
        let json = String::from_utf8(plain).map_err(|e| OpenError::Damaged(e.to_string()))?;
        return Ok((json, Protection { key, slots: s.slots.clone(), access: slot.role }));
    }
    Err(OpenError::WrongPassword)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cheap parameters for the tests (the real ones take a while in a debug build); one test uses the real ones.
    const FAST: Kdf = Kdf { m_kib: 8 * 1024, t: 1, p: 1 };
    const JSON: &str = r#"{"format":1,"scene":{"shapes":[{"name":"SecretGear"}]}}"#;

    /// A password for the tests, made when they run (no password is written into the code): `tag` and the number of the process.
    pub(crate) fn pw(tag: &str) -> String {
        format!("{tag}-{}", std::process::id())
    }

    /// A password of `n` characters, made when the tests run.
    fn pw_len(n: usize) -> String {
        std::process::id().to_string().chars().cycle().take(n).collect()
    }

    #[test]
    fn both_passwords_open_the_project_with_their_rights() {
        let p = Protection::with_kdf(&pw("owner"), Some(&pw("open")), FAST).unwrap();
        let sealed = p.seal(JSON);
        assert!(is_sealed(&sealed));
        assert!(!is_sealed(JSON));
        assert!(!sealed.contains("SecretGear"), "the content is not readable");
        let (json, o) = open(&sealed, &pw("owner")).unwrap();
        assert_eq!((json.as_str(), o.access), (JSON, Access::Owner));
        let (json, r) = open(&sealed, &pw("open")).unwrap();
        assert_eq!((json.as_str(), r.access), (JSON, Access::Open));
        // A save by the person with the open password keeps both passwords.
        let again = r.seal(r#"{"changed":true}"#);
        assert_eq!(open(&again, &pw("owner")).unwrap().0, r#"{"changed":true}"#);
        assert_eq!(open(&again, &pw("open")).unwrap().1.access, Access::Open);
        assert_eq!(open(&sealed, &pw("wrong")).unwrap_err(), OpenError::WrongPassword);
    }

    #[test]
    fn the_owner_password_unlocks_a_project_opened_with_the_open_one() {
        let p = Protection::with_kdf(&pw("owner"), Some(&pw("open")), FAST).unwrap();
        let (_, mut r) = open(&p.seal(JSON), &pw("open")).unwrap();
        assert_eq!(r.unlock_owner(&pw("open")).unwrap_err(), OpenError::WrongPassword, "the open password is not the owner's");
        assert_eq!(r.access, Access::Open);
        r.unlock_owner(&pw("owner")).unwrap();
        assert_eq!(r.access, Access::Owner);
        assert_eq!(open(&r.seal(JSON), &pw("open")).unwrap().1.access, Access::Open, "both passwords stay");
    }

    #[test]
    fn only_an_owner_password_lets_only_the_owner_in() {
        let p = Protection::with_kdf(&pw("owner"), None, FAST).unwrap();
        assert!(!p.has_open_password());
        let sealed = p.seal(JSON);
        assert_eq!(open(&sealed, &pw("owner")).unwrap().1.access, Access::Owner);
        assert_eq!(open(&sealed, &pw("someone")).unwrap_err(), OpenError::WrongPassword);
    }

    #[test]
    fn every_save_has_a_new_nonce_and_a_changed_file_does_not_open() {
        let p = Protection::with_kdf(&pw("owner"), Some(&pw("open")), FAST).unwrap();
        let (a, b) = (p.seal(JSON), p.seal(JSON));
        assert_ne!(a, b, "the same project seals differently every time");
        let mut v: serde_json::Value = serde_json::from_str(&a).unwrap();
        // A flipped byte of the content.
        let mut data = B64.decode(v["data"].as_str().unwrap()).unwrap();
        data[3] ^= 1;
        let mut changed = v.clone();
        changed["data"] = B64.encode(&data).into();
        assert!(matches!(open(&changed.to_string(), &pw("owner")), Err(OpenError::Damaged(_))));
        // The roles swapped: the open password would claim to be the owner's.
        let slots = v["slots"].as_array_mut().unwrap();
        slots[0]["role"] = "open".into();
        slots[1]["role"] = "owner".into();
        assert!(open(&v.to_string(), &pw("open")).is_err());
        // Parameters too weak or too big are refused before any key is made.
        let mut weak: serde_json::Value = serde_json::from_str(&a).unwrap();
        weak["slots"][0]["kdf"]["m_kib"] = 8.into();
        assert!(matches!(open(&weak.to_string(), &pw("owner")), Err(OpenError::Damaged(_))));
        assert!(matches!(open("{\"protected_cre\":2}", &pw("x")), Err(OpenError::Damaged(_))));
        assert!(matches!(open("not json", &pw("x")), Err(OpenError::Damaged(_))));
    }

    #[test]
    fn the_passwords_must_be_long_enough_and_differ() {
        let (long, other) = (pw_len(MIN_PASSWORD), format!("{}x", pw_len(MIN_PASSWORD)));
        assert!(check_passwords(&pw_len(MIN_PASSWORD - 1), None).is_err());
        assert!(check_passwords(&long, Some(&pw_len(4))).is_err());
        assert!(check_passwords(&long, Some(&long)).is_err());
        assert!(check_passwords(&long, Some(&String::new())).is_ok());
        assert!(check_passwords(&long, Some(&other)).is_ok());
        // Characters, not bytes: eight Polish letters are long enough.
        let polish: String = std::process::id().to_string().chars().map(|_| 'ż').chain(std::iter::repeat_n('ą', MIN_PASSWORD)).take(MIN_PASSWORD).collect();
        assert!(check_passwords(&polish, None).is_ok());
    }

    /// The real parameters work (and are not absurdly slow even unoptimised).
    #[test]
    fn the_real_key_parameters_open_a_file() {
        let t = std::time::Instant::now();
        let p = Protection::new(&pw("owner"), None).unwrap();
        let sealed = p.seal(JSON);
        assert_eq!(open(&sealed, &pw("owner")).unwrap().0, JSON);
        assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
    }
}
