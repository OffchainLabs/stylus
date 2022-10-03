;; Copyright 2022, Offchain Labs, Inc.
;; For license information, see https://github.com/nitro/blob/master/LICENSE

(module
    (type (;0;) (func (param i32)))
    (type (;1;) (func (param i32 i32 i32)))
    (import "arbitrator_forward__polyglot" "read_args" (func $read_args (type 0)))
    (import "arbitrator_forward__polyglot" "return_data" (func $return_data (type 1)))
    (export "env__read_args" (func $read_args))
    (export "env__return_data" (func $return_data)))
