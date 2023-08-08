;; Copyright 2023, Offchain Labs, Inc.
;; For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

(module
    (import "hostio" "wavm_link_module" (func $link (param i32) (result i32)))
    (import "hostio" "wavm_unlink_module" (func $unlink (param) (result)))
    (data (i32.const 0x000)
         "\7a\cd\ed\ed\a3\18\fa\b7\4b\21\35\f5\08\8f\37\02\f3\96\fa\70\79\5d\c0\c9\55\92\8b\42\77\58\09\95")
    (data (i32.const 0x020)
         "\91\3a\58\fe\e6\8f\2f\3f\a4\d7\a6\60\7e\f8\06\8a\db\de\37\31\e8\66\99\9a\f5\2b\00\39\b0\df\f0\29")
    (data (i32.const 0x040)
         "\94\63\42\17\b2\0a\c3\94\ce\e1\0c\50\74\7b\fb\2c\22\3f\ca\97\3e\18\30\90\df\9b\36\37\40\fa\57\df")
    (data (i32.const 0x060)
         "\f1\37\72\b3\30\37\09\8e\b2\d5\e5\49\20\57\ea\17\1d\a6\1c\85\58\82\e1\96\c7\80\a2\e9\26\e7\fe\23")
    (data (i32.const 0x080)
         "\03\a1\b5\e0\fa\39\f0\89\12\a4\f7\af\fe\af\5b\21\e1\61\1f\4b\f7\fb\51\c5\e7\07\6b\12\cd\b6\fd\ad")
    (data (i32.const 0x0a0)
         "\b0\53\f4\29\a3\9d\e0\2d\d4\7f\c9\d4\fa\0d\e8\ed\ff\90\c2\c2\e8\f4\9d\41\3d\04\67\2a\75\2d\ff\d2")
    (data (i32.const 0x0c0)
         "\c3\4c\a0\04\4e\cb\62\af\fb\15\f9\da\08\22\91\28\be\97\fa\04\f4\da\a9\1b\07\de\6d\79\f1\af\27\30")
    (data (i32.const 0x0e0)
         "\e0\44\10\9b\5d\95\19\8b\e0\5c\20\fe\31\ce\f0\cf\b7\c7\6a\9d\a9\88\dd\4d\a9\fd\78\ff\5f\69\dd\82")
    (data (i32.const 0x100)
         "\ef\75\83\c3\de\bf\e1\41\0a\3d\24\81\1c\a3\b7\4d\a5\18\3b\46\63\f2\5d\c9\14\d3\70\aa\b8\b3\21\56")
    (data (i32.const 0x120)
         "\c9\a6\64\85\89\33\76\a1\8f\17\51\9a\2d\74\b1\9b\89\16\0d\ea\87\3b\ed\f4\62\b7\e5\1b\aa\13\02\8f")
    (data (i32.const 0x140)
         "\e2\b4\0f\72\1d\71\60\c7\5b\37\ce\89\7e\de\e1\ab\08\ed\27\d0\06\0e\55\a3\9c\44\e3\4b\3c\b9\54\86")
    (data (i32.const 0x160)
         "\fc\39\b7\9f\6a\2b\66\33\8f\2c\a5\9b\33\67\a7\f4\45\5e\0b\34\49\6d\1a\6c\a6\c1\7e\9a\7e\91\27\d7")
    (data (i32.const 0x180)
         "\16\c7\a1\3d\ad\27\8f\a9\11\7c\6e\0c\a1\4c\68\ce\46\c6\c0\e2\cf\97\45\88\53\08\22\17\33\0b\8c\fb")

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
