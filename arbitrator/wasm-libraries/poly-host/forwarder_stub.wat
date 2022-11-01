;; Copyright 2022, Offchain Labs, Inc.
;; For license information, see https://github.com/nitro/blob/master/LICENSE

(module
    (type (;0;) (func (param i32)))
    (type (;1;) (func (param i32 i32)))
    (func $read_args (export "env__read_args") (type 0) (param i32)
          unreachable)
    (func $return_data (export "env__return_data") (type 1) (param i32 i32)
          unreachable))
