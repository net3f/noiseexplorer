/* ---------------------------------------------------------------- *
 * PRIMITIVES                                                       *
 * ---------------------------------------------------------------- */

use crate::{
	consts::{BLOCKLEN, DHLEN, HASHLEN, MAC_LENGTH, NONCE_LENGTH},
	types::Keypair,
};
use byteorder::{ByteOrder, LittleEndian};
use crypto::{blake2s::Blake2s, digest::Digest};
use hacl_star::chacha20poly1305;

#[allow(dead_code)]
pub fn generate_keypair() -> Keypair {
	Keypair::new()
}

pub fn encrypt(k: [u8; DHLEN], n: u64, ad: &[u8], plaintext: &[u8]) -> Vec<u8> {
	let mut mac: [u8; MAC_LENGTH] = [0u8; MAC_LENGTH];
	let mut in_out = plaintext.to_owned();
	let mut nonce: [u8; NONCE_LENGTH] = [0u8; NONCE_LENGTH];
	LittleEndian::write_u64(&mut nonce[4..], n);
	chacha20poly1305::key(&k)
		.nonce(&nonce)
		.encrypt(&ad, &mut in_out[..], &mut mac);
	let mut ciphertext: Vec<u8> = in_out;
	let mut tag: Vec<u8> = Vec::from(&mac[..]);
	ciphertext.append(&mut tag);
	ciphertext
}

pub fn decrypt(k: [u8; DHLEN], n: u64, ad: &[u8], ciphertext: &[u8]) -> Option<Vec<u8>> {
	let temp = Vec::from(ciphertext);
	let (x, y) = temp.split_at(temp.len() - MAC_LENGTH);
	let mut in_out = x.to_owned();
	let mut mac: [u8; MAC_LENGTH] = [0u8; MAC_LENGTH];
	mac.copy_from_slice(y);
	let mut nonce: [u8; NONCE_LENGTH] = [0u8; NONCE_LENGTH];
	LittleEndian::write_u64(&mut nonce[4..], n);
	let decryption_status =
		chacha20poly1305::key(&k)
			.nonce(&nonce)
			.decrypt(&ad, &mut in_out[..], &mac);
	if decryption_status {
		Some(in_out)
	} else {
		None
	}
}

pub fn hash(data: &[u8]) -> Vec<u8> {
	let mut blake2s: Blake2s = Blake2s::new(HASHLEN);
	blake2s.input(&data[..]);
	let digest_res: &mut [u8] = &mut [0u8; HASHLEN];
	blake2s.result(digest_res);
	blake2s.reset();
	Vec::from(digest_res)
}

pub fn hmac(key: &[u8], data: &[u8], out: &mut [u8]) {
	let mut blake2s: Blake2s = Blake2s::new(HASHLEN);
	let mut ipad = [0x36u8; BLOCKLEN];
	let mut opad = [0x5cu8; BLOCKLEN];
	for count in 0..key.len() {
		ipad[count] ^= key[count];
		opad[count] ^= key[count];
	}
	blake2s.reset();
	blake2s.input(&ipad[..BLOCKLEN]);
	blake2s.input(data);
	let mut inner_output = [0u8; HASHLEN];
	blake2s.result(&mut inner_output);
	blake2s.reset();
	blake2s.input(&opad[..BLOCKLEN]);
	blake2s.input(&inner_output[..HASHLEN]);
	blake2s.result(out);
}

pub fn hkdf(
	chaining_key: &[u8],
	input_key_material: &[u8],
	outputs: usize,
	out1: &mut [u8],
	out2: &mut [u8],
	out3: &mut [u8],
) {
	let mut temp_key = [0u8; HASHLEN];
	hmac(chaining_key, input_key_material, &mut temp_key);
	hmac(&temp_key, &[1u8], out1);
	if outputs == 1 {
		return;
	}
	let mut in2 = [0u8; HASHLEN + 1];
	copy_slices!(&out1[0..HASHLEN], &mut in2);
	in2[HASHLEN] = 2;
	hmac(&temp_key, &in2[..=HASHLEN], out2);
	if outputs == 2 {
		return;
	}
	let mut in3 = [0u8; HASHLEN + 1];
	copy_slices!(&out2[0..HASHLEN], &mut in3);
	in3[HASHLEN] = 3;
	hmac(&temp_key, &in3[..=HASHLEN], out3);
}
