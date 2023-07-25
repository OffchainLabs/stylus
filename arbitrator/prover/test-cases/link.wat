;; Copyright 2023, Offchain Labs, Inc.
;; For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

(module
    (import "hostio" "wavm_link_module" (func $link (param i32) (result i32)))
    (import "hostio" "wavm_unlink_module" (func $unlink (param) (result)))
    (data (i32.const 0x000)
         "\2a\94\85\60\92\8c\fb\08\cf\0e\e0\c4\9c\02\5b\0f\03\b1\3d\b9\ac\83\f7\f8\3d\9a\8c\9b\c3\01\ec\24")
    (data (i32.const 0x020)
         "\40\49\8b\d3\0d\81\e6\61\ef\f7\99\1e\6f\87\33\24\d5\48\4f\8e\c4\28\9b\ff\44\34\3a\62\15\5f\65\f3")
    (data (i32.const 0x040)
         "\ba\f4\d6\92\3a\dc\6d\01\b7\a7\f7\8b\bc\26\4e\b6\73\e7\1c\0b\6a\1b\e9\43\cf\aa\78\51\7d\f0\6e\e2")
    (data (i32.const 0x060)
         "\8c\bd\e0\cc\72\d6\c7\16\9b\4a\a0\16\dc\10\62\b9\01\20\6f\57\ea\f9\d2\e9\65\41\ab\f6\99\9c\fe\ab")
    (data (i32.const 0x080)
         "\8e\ef\66\f7\31\69\77\15\a5\d1\37\e8\5f\1b\40\b2\d9\cc\87\12\bf\cd\33\e9\17\0e\29\16\3b\ae\5a\89")
    (data (i32.const 0x0a0)
         "\29\87\e2\17\e8\2c\fb\3a\02\a1\92\ce\e9\43\a5\da\91\93\af\80\80\7b\5f\d3\cf\e8\cb\ee\64\bf\84\1d")
    (data (i32.const 0x0c0)
         "\3e\58\e3\ac\66\62\b8\93\eb\6e\da\57\3a\44\bf\c7\05\4e\7b\0d\32\90\3e\38\c5\88\d2\58\0c\ad\fe\34")
    (data (i32.const 0x0e0)
         "\2d\a1\42\a0\b6\8c\4b\f4\55\61\82\27\15\15\d5\16\20\b3\37\10\f2\33\c3\01\1f\36\ea\56\0f\b5\f4\ce")
    (data (i32.const 0x100)
         "\a2\2b\96\13\c4\76\97\af\01\19\a6\a3\3c\75\6a\52\bb\74\99\e9\55\7a\dc\2c\d6\66\e5\f0\ac\a2\c1\84")
    (data (i32.const 0x120)
         "\39\34\5a\b8\c7\cd\6a\cc\d3\45\00\a5\72\67\02\e2\8d\7f\22\f8\81\8f\50\60\d0\32\40\70\74\31\6a\72")
    (data (i32.const 0x140)
         "\a4\11\ca\e6\37\2b\70\48\db\70\7f\d2\0e\f1\c6\73\dd\21\f3\8d\91\14\f7\44\29\93\38\d1\43\b5\8b\e7")
    (data (i32.const 0x160)
         "\ae\01\43\c4\fa\31\c1\30\eb\d6\16\53\7b\a9\e4\0d\0c\52\ab\4b\2a\4c\fe\62\06\5e\e8\33\c8\f4\d4\d4")
    (data (i32.const 0x180)
         "\b0\11\13\06\77\24\48\82\59\c7\12\ba\43\bf\43\02\89\c5\cd\c9\3a\c5\ba\7a\92\c8\88\a0\5a\49\4e\9e")

    (func $start (local $counter i32)

         ;; add modules
         (loop $top
             ;; increment counter
             local.get $counter
             local.get $counter
             i32.const 1
             i32.add
             local.set $counter

             ;; link module with unique hash
             i32.const 32
             i32.mul
             call $link

             ;; loop until 12 modules
             i32.const 12
             i32.le_s
             br_if $top
         )

         ;; reset counter
         i32.const 0
         local.set $counter

         ;; link and unlink modules
         (loop $top
             ;; increment counter
             local.get $counter
             local.get $counter
             i32.const 1
             i32.add
             local.set $counter

             ;; unlink 2 modules
             call $unlink
             call $unlink

             ;; link module with unique hash
             i32.const 32
             i32.mul
             call $link

             ;; loop until most are gone
             i32.const 3
             i32.ge_s
             br_if $top))
    (memory 1)
    (start $start))
