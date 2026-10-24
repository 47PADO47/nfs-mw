# Multiplayer protocol: handshake and sealed packets

Status: **built** 2026-10-22 in `libs/blackbox-net-core` (module `secure`, cargo feature `netcode`, and the seal slot of
`conn`). Sections 4 and 5.1 of [protocol](protocol.md) point here; the design reasons are in [security](security.md) section 4
and the caps in [operations security](security-ops.md) section 2. The code and its module docs are the reference; this file
records the layout as built and every place it differs from the earlier design text.

## 1. Byte 0 of every datagram

| Bit | Handshake packet | Data packet |
|---|---|---|
| 7 | 1 | 0 |
| 6 to 4 | `hs_type` | bits 6 and 5: key phase; bit 4: reserved 0 |
| 3 to 0 | packet format 0 | bits 3 to 1: format 0; bit 0: the ack block is valid (the sender has received a packet) |

`hs_type`: 0 `Hello`, 1 `Challenge`, 2 `HelloCookie`, 3 `NoiseMsg2`, 4 `NoiseMsg3`, 5 `Reject`; 6 and 7 are invalid.
`secure::classify` sorts any datagram by this byte before anything else is decoded.

## 2. Handshake packets

All integers little-endian. Every decoder checks the exact length first, then the fields, then that padding is zero.

| Packet | Dir | Fields after byte 0 | Size |
|---|---|---|---|
| `Hello` | C to S | `proto_major` u8, `format` u8 (= 1), `client_nonce` 16, `client_conn_id` u16, zero padding | exactly 512 |
| `Challenge` | S to C | `server_conn_id` u16, `bucket` u8, `cookie` 32 | exactly 36 |
| `HelloCookie` | C to S | `proto_major`, `format`, `pattern` u8 (0 XX, 1 XK), `client_nonce` 16, `client_conn_id` u16, `server_conn_id` u16, `bucket` u8, `cookie` 32, `noise_len` u16 (1 to 256), Noise message 1, zero padding | exactly 512 |
| `NoiseMsg2` | S to C | `client_conn_id` u16, `len` u16 (1 to 256), Noise message 2; nothing after it | 5 + len (101 for XX) |
| `NoiseMsg3` | C to S | `server_conn_id` u16, `len` u16 (1 to 256), Noise message 3; nothing after it | 5 + len (69 for XX) |
| `Reject` | S to C | `reason` u8: 1 full, 2 protocol major unsupported, 3 shutting down | exactly 2 |

`Challenge` is 36 bytes: the 37 in the earlier table was an arithmetic slip (1 + 2 + 1 + 32).

## 3. Cookie

`cookie = BLAKE2s-256-MAC(secret, "cookie/1" || address || client_nonce || client_conn_id || server_conn_id || bucket)`, where
`address` is the IPv6 address (IPv4 mapped) and port (18 bytes) and `bucket` is `now_ms / 15000` as a u64. The `Challenge`
carries the low 8 bits of the bucket; the server accepts the current and the previous bucket, so a cookie lives 15 to 30 s. The
secret rotates every hour (`HandshakeServer::expire` draws a new one), and the previous secret is kept. BLAKE2s is used because
the Noise suite already brings it; the design said "HMAC" generically. Binding the two connection ids means a `HelloCookie` cannot
claim a server id the server did not offer to that address.

## 4. Flow

1. The client sends `Hello`. The server, if its gate allows (not over 32 half-open, not over 8 new handshakes in the last second,
   not an ignored address), picks a free connection id and answers `Challenge`. It keeps nothing.
2. The client answers `HelloCookie` with Noise message 1. The server checks, in this order: the cookie (constant time; a bad one
   is dropped and counted); a running slot with that server id (the same client gets the same `NoiseMsg2` again, within the
   amplification limit); the id is not an established connection; the protocol major (`Reject` 2); shutting down (`Reject` 3);
   the pattern is accepted (dropped otherwise); the connection cap (`Reject` 1); the per-address handshake rate and connection cap
   and the half-open cap (dropped). Only then does it allocate a slot and run Noise.
3. The server answers `NoiseMsg2`; the client answers `NoiseMsg3` and now holds the keys. The client resends `NoiseMsg3` until
   the first sealed packet from the server opens (`HandshakeClient::confirm`). The server's first sealed packet confirms the keys;
   the caller sends one right after `ServerAction::Established`.
4. A slot that does not finish in 5 s is freed. Until `NoiseMsg3` verifies, a slot sends at most 3 times the bytes it received.

**Retries.** Each client stage sends at once, then after waits of 0.5, 0.5, 1 and 2 s (sends at 0, 0.5, 1 and 2 s) and reports
"no answer" at 4 s. The client nonce is drawn **once per handshake**, not per `Hello` resend as protocol.md first said: a
`Challenge` does not name the `Hello` it answers, so a late answer to an earlier send must still match the nonce the client echoes.

**Noise.** `Noise_XX_25519_ChaChaPoly_BLAKE2s` on a LAN, `Noise_XK_25519_ChaChaPoly_BLAKE2s` against a pinned key, with `snow`
0.10 and its RustCrypto resolver. The prologue is passed in by the game (`nfsmw-mp/<major>` there; the library has no game
names). The handshake messages carry no payload, so there is no early data. After message 3 both sides take the two transport
keys with `snow`'s raw split.

## 5. Sealed packet

| Offset | Size | Field | Sealed? |
|---|---|---|---|
| 0 | 1 | flags (section 1) | no, authenticated |
| 1 | 2 | connection id (of the receiver) | no, authenticated |
| 3 | 4 | packet number (low 32 bits) | no, authenticated |
| 7 | 4 | `ack` | yes |
| 11 | 4 | `ack_bits` | yes |
| 15 | n | frames, at most 1,169 bytes | yes |
| 15 + n | 16 | ChaCha20-Poly1305 tag | |

- **Keys and nonce.** One key per direction from the handshake. The nonce is the packet number expanded to 64 bits (the
  candidate nearest to the next expected number, as QUIC does), laid out as Noise's `ChaChaPoly` nonce. A sender refuses to
  seal a number not above the last one, so a nonce never repeats under one key. `snow`'s own transport state is not used for
  packets because it takes no associated data and cannot hold two keys side by side.
- **Receive order.** The receiver checks the handshake bit, the connection id and whether the receive window (1,024 numbers)
  would take the number, then checks the tag, and only then records the number. A forged packet changes nothing; a replayed one
  is a duplicate; one more than 1,024 behind is too old.
- **Rekey.** After 2^24 packets or one hour the sender moves to the next key phase with Noise `REKEY` (encrypt 32 zero bytes at
  nonce 2^64 - 1). A receiver that sees the next phase derives the next key; the old key stays for 3 s for reordered packets,
  then is erased. A phase two ahead, or an erased old phase, is dropped and counted.
- **Failure** is silent: no reply, a counter (`SealStats`).
- **In memory.** The listen host's own client uses `conn::Plain`: no tag, no handshake (ADR 0009 section 3).

## 6. Not built yet

- The `Auth` message, its 5 s deadline and the resume token: game protocol, in `nfsmw-net`.
- Carrying the connect token and the PAKE messages on the control channel: the library opens and checks them
  (`secure::token`, `secure::password`); the game sends them.
- Sockets with 256 KiB buffers. The net thread itself is built (module `endpoint`, [integration](integration.md) section 1): it
  runs this handshake on a real socket, sends the server's first sealed packet right away and then every 100 ms until the client
  is heard from, and drops data packets from any address other than the handshake's.
