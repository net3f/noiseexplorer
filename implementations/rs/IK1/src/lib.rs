/*
IK1:
  <- s
  ...
  -> e, s
  <- e, ee, se, es
  ->
  <-

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