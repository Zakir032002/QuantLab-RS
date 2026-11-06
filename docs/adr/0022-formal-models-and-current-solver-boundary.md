# ADR-0022: formal models and the current solver boundary

Status: source implemented, verification blocked; no proof pass.

K01 retains full i128 operands and scales 0..18, including the established
alignment-overflow contract. Full symbolic scales timed out after 300 seconds;
checking all 361 scale pairs with the same full value domain also timed out.
R5 stops CP3.11. Raw logs and tool/host metadata are in evidence/part3/proofs.
No smaller numeric domain is substituted to manufacture a pass.

K02's independent base-2^32 reference covers signed operands of magnitude below
2^80, all scale combinations and all rounding modes, including wide products.
It does not call the product's base-2^64 U256 routines. Its unwind bound covers
the actual 256-step divider. K03/K04 model exact round trips and a division-based
mathematical order independently of wide multiplication. These have not run.

K05 belongs in qlrs-io because literal-length policy is an I/O responsibility;
putting it in num would reverse dependency direction. It exercises actual
Limits.number and Fixed.parse on up to eight bytes with a symbolic length cap.
K06 covers arbitrary initial bitsets, four symbolic updates and N <=130 (three
64-bit words). K07 uses the actual checked prefix helper used by the reducer,
with up to eight full-width symbolic i128 entries. These have not run.

Native numeric/BigInt differential and corpus tests remain active. They are not
represented as formal proofs. The Kani driver does not expose Cargo's --locked
flag; lock hashes and exact versions are recorded, and ordinary Cargo commands
continue to use --locked.

The user's November 6 "Continue" reopened proof work. Splitting K01 into 361
constant-scale queries retains every pair of scales and full i128 operands.
The hardest-distance probe (left scale 0, right scale 18) passed 219 checks in
163.84747 seconds, using CaDiCaL; see proofs/K01-split-probe. The full matrix
and remaining harnesses are now running. A single probe is not a K01 pass.
