/*
Kpsk0:
  -> s
  <- s
  ...
  -> psk, e, es, ss
*/

/* ---------------------------------------------------------------- *
 * PARAMETERS                                                       *
 * ---------------------------------------------------------------- */

#[macro_use]
pub(crate) mod macros;

pub(crate) mod consts;
pub(crate) mod prims;
pub(crate) mod state;

pub mod noisesession;
pub mod types;