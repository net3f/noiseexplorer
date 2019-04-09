/* ---------------------------------------------------------------- *
 * STATE MANAGEMENT                                                 *
 * ---------------------------------------------------------------- */

use crate::{
	consts::{DHLEN, EMPTY_HASH, EMPTY_KEY, HASHLEN, NONCE_LENGTH, ZEROLEN},
	prims::{decrypt, encrypt, hash, hkdf},
	types::{Hash, Key, Keypair, MessageBuffer, Nonce, Psk, PublicKey},
};
use hacl_star::chacha20poly1305;

fn from_slice_hashlen(bytes: &[u8]) -> [u8; HASHLEN] {
	let mut array = EMPTY_HASH;
	let bytes = &bytes[..array.len()];
	array.copy_from_slice(bytes);
	array
}

#[derive(Clone)]
pub(crate) struct CipherState {
	k: Key,
	n: Nonce,
}

impl CipherState {
	pub(crate) fn new() -> CipherState {
		CipherState::from_key(Key::new())
	}
	pub(crate) fn clear_key(&mut self) {
		self.k.clear();
	}
	pub(crate) fn from_key(k: Key) -> CipherState {
		let nonce: Nonce = Nonce::new();
		CipherState { k: k, n: nonce }
	}
	pub(crate) fn has_key(&self) -> bool {
		!self.k.is_empty()
	}
	#[allow(dead_code)]
	pub(crate) fn set_nonce(&mut self, n: Nonce) {
		self.n = n;
	}
	#[allow(dead_code)]
	pub(crate) fn get_nonce(&self) -> Nonce {
		self.n
	}
	pub(crate) fn encrypt_with_ad(&mut self, ad: &[u8], plaintext: &[u8]) -> Vec<u8> {
		if !self.has_key() {
			Vec::from(plaintext)
		} else {
			let ciphertext: Vec<u8> = encrypt(
				from_slice_hashlen(&self.k.as_bytes()[..]),
				self.n.get_value(),
				ad,
				plaintext,
			);
			self.n.increment();
			ciphertext
		}
	}
	pub(crate) fn decrypt_with_ad(&mut self, ad: &[u8], ciphertext: &[u8]) -> Option<Vec<u8>> {
		if !self.has_key() {
			Some(Vec::from(ciphertext))
		} else if let Some(plaintext) = decrypt(
			from_slice_hashlen(&self.k.as_bytes()[..]),
			self.n.get_value(),
			ad,
			ciphertext,
		) {
			self.n.increment();
			Some(plaintext)
		} else {
			println!("Unsuccessful Decryption, problem with ad, nonce not incremented\n\nDECRYPT({:X?}, {:X?}, {:X?}, {:X?})", from_slice_hashlen(&self.k.as_bytes()[..]), self.n.get_value(), ad, ciphertext);
			None
		}
	}
	#[allow(dead_code)]
	pub(crate) fn rekey(&mut self) {
		let mut in_out = EMPTY_KEY;
		chacha20poly1305::key(&self.k.as_bytes())
			.nonce(&[0xFFu8; NONCE_LENGTH])
			.encrypt(&ZEROLEN[..], &mut in_out[..], &mut [0u8; 16]);
		self.k.clear();
		self.k = Key::from_bytes(in_out);
	}
	pub(crate) fn write_message_regular(&mut self, payload: &[u8]) -> MessageBuffer {
		MessageBuffer {
			ne: EMPTY_KEY,
			ns: Vec::new(),
			ciphertext: self.encrypt_with_ad(&ZEROLEN[..], payload),
		}
	}
	pub(crate) fn read_message_regular(&mut self, message: &MessageBuffer) -> Option<Vec<u8>> {
		self.decrypt_with_ad(&ZEROLEN[..], &message.ciphertext)
	}
}

#[derive(Clone)]
pub struct SymmetricState {
	cs: CipherState,
	ck: Hash,
	h: Hash,
}

impl SymmetricState {
	pub(crate) fn clear(&mut self) {
		self.cs.clear_key();
		self.ck.clear();
		self.h.clear();
	}
	pub fn initialize_symmetric(protocol_name: &[u8]) -> SymmetricState {
        let h: Hash = Hash::from_bytes(if protocol_name.len() <= 32 {
                let mut array = [0u8; HASHLEN];
                copy_slices!(protocol_name,&mut array);
                array
        } else {
                hash(&[protocol_name])
        });
        let ck: Hash = Hash::from_bytes(from_slice_hashlen(&h.as_bytes()[..]));
		let cs: CipherState = CipherState::new();
		SymmetricState { cs, ck, h }
	}
	pub(crate) fn mix_key(&mut self, input_key_material: &[u8]) {
		let mut out0: [u8; HASHLEN] = EMPTY_HASH;
		let mut out1: [u8; HASHLEN] = EMPTY_HASH;
		let mut out2: [u8; HASHLEN] = EMPTY_HASH;
		hkdf(
			&self.ck.as_bytes()[..],
			input_key_material,
			2,
			&mut out0[..],
			&mut out1[..],
			&mut out2[..],
		);
		self.ck = Hash::from_bytes(out0);
		let mut temp_k: [u8; 32] = EMPTY_KEY;
		temp_k.copy_from_slice(&out1[..32]);
		self.cs = CipherState::from_key(Key::from_bytes(temp_k));
	}
	pub(crate) fn mix_hash(&mut self, data: &[u8]) {
        self.h = Hash::from_bytes(hash(&[&self.h.as_bytes()[..], data]));
	}
	#[allow(dead_code)]
	pub(crate) fn mix_key_and_hash(&mut self, input_key_material: &[u8]) {
		let mut out0: [u8; HASHLEN] = EMPTY_HASH;
		let mut out1: [u8; HASHLEN] = EMPTY_HASH;
		let mut out2: [u8; HASHLEN] = EMPTY_HASH;
		hkdf(
			&self.ck.as_bytes()[..],
			input_key_material,
			3,
			&mut out0[..],
			&mut out1[..],
			&mut out2[..],
		);
		self.ck = Hash::from_bytes(out0);
		let temp_h: [u8; HASHLEN] = out1;
		let mut temp_k: [u8; DHLEN] = out2;
		self.mix_hash(&temp_h[..]);
		temp_k.copy_from_slice(&out2[..32]);
		self.cs = CipherState::from_key(Key::from_bytes(temp_k));
	}
	#[allow(dead_code)]
	pub(crate) fn get_handshake_hash(&self) -> [u8; HASHLEN] {
		from_slice_hashlen(&self.h.as_bytes()[..])
	}
	pub(crate) fn encrypt_and_hash(&mut self, plaintext: &[u8]) -> Option<Vec<u8>> {
		let ciphertext: Vec<u8> = self.cs.encrypt_with_ad(&self.h.as_bytes()[..], plaintext);
		self.mix_hash(&ciphertext);
		Some(ciphertext)
	}
	pub(crate) fn decrypt_and_hash(&mut self, ciphertext: &[u8]) -> Option<Vec<u8>> {
		if let Some(plaintext) = self.cs.decrypt_with_ad(&self.h.as_bytes()[..], &ciphertext) {
			self.mix_hash(ciphertext);
			return Some(Vec::from(&plaintext[..]));
		} else {
			panic!("Invalid ad");
		}
	}
	pub(crate) fn split(&mut self) -> (CipherState, CipherState) {
		let mut temp_k1: [u8; HASHLEN] = EMPTY_HASH;
		let mut temp_k2: [u8; HASHLEN] = EMPTY_HASH;
		let mut out2: [u8; HASHLEN] = EMPTY_HASH;
		hkdf(
			&self.ck.as_bytes()[..],
			&ZEROLEN[..],
			2,
			&mut temp_k1[..],
			&mut temp_k2[..],
			&mut out2[..],
		);
		let cs1: CipherState =
			CipherState::from_key(Key::from_bytes(from_slice_hashlen(&temp_k1[..32])));
		let cs2: CipherState =
			CipherState::from_key(Key::from_bytes(from_slice_hashlen(&temp_k2[..32])));
		(cs1, cs2)
	}
}

#[derive(Clone)]
pub struct HandshakeState {
	ss: SymmetricState,
	s: Keypair,
	e: Keypair,
	rs: PublicKey,
	re: PublicKey,
	psk: Psk,
}

/* HandshakeState */
impl HandshakeState {
	pub(crate) fn clear(&mut self) {
        self.s.clear();
        self.e.clear();
        self.rs.clear();
        self.re.clear();
        self.psk.clear();
    }
	pub(crate) fn set_ephemeral_keypair(&mut self, e: Keypair) {
        self.e = e;
    }
/* $NOISE2RS_I$ */
/* $NOISE2RS_W$ */
/* $NOISE2RS_R$ */
}

#[test]
fn initkey_test() {
	CipherState::new();
}
