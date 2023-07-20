;; Copyright 2023, Offchain Labs, Inc.
;; For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

(module
    (import "hostio" "wavm_link_module" (func $link (param i32) (result i32)))
    (import "hostio" "wavm_unlink_module" (func $unlink (param) (result)))
    (data (i32.const 0x000)
         "\3e\2b\a8\7c\04\f9\c9\36\57\b8\a8\20\eb\b4\74\13\11\b2\36\48\8e\01\36\50\b3\59\d4\50\b7\15\86\a1")
    (data (i32.const 0x020)
         "\cd\ef\2c\33\e2\88\88\0f\95\e5\56\f6\f2\09\eb\c1\12\6c\5e\d3\3e\f1\3a\ee\b1\de\6a\6b\1d\72\55\0a")
    (data (i32.const 0x040)
         "\e9\a4\72\b9\ac\2c\39\c3\bd\07\ba\2a\07\27\01\37\1f\0d\4a\4f\32\37\b4\9b\76\b2\2f\82\75\c5\e5\db")
    (data (i32.const 0x060)
         "\0a\3e\95\a0\01\bf\38\a7\f9\c0\4b\74\53\06\fc\a9\be\6b\4a\84\65\7e\04\b1\43\12\d9\e3\fd\5b\4d\ab")
    (data (i32.const 0x080)
         "\17\38\60\10\80\4d\6b\4e\04\51\1f\af\b8\5f\73\a3\97\70\6f\e4\ce\0e\a4\c2\1a\e0\b5\96\32\d2\e7\0b")
    (data (i32.const 0x0a0)
         "\04\b3\01\ae\24\d9\10\12\fe\be\a3\2a\0f\ee\8c\20\6a\bc\af\d4\40\c0\70\62\d9\7f\40\ae\fb\f1\41\cd")
    (data (i32.const 0x0c0)
         "\8f\0a\38\39\b9\f2\d7\ba\3f\24\53\59\02\c6\f6\4e\c5\f1\5c\5d\cf\16\8f\9b\6d\7e\2b\ec\cc\c8\90\c2")
    (data (i32.const 0x0e0)
         "\7f\73\86\67\a0\b5\c3\85\43\a4\4d\05\ab\9e\1d\9b\68\c5\c8\cc\03\f4\fa\5b\98\64\27\6e\e6\26\5e\c0")
    (data (i32.const 0x100)
         "\4b\c3\5f\e7\d7\63\0e\13\29\e4\a7\c0\7b\cb\5c\b4\44\d8\d7\b0\53\52\d4\d3\64\90\7f\35\d3\a8\25\53")
    (data (i32.const 0x120)
         "\6c\bc\c4\8f\38\8c\9f\f3\e5\74\67\35\b4\9f\63\c9\f2\8f\71\b1\12\d7\60\3d\d9\4d\5f\0c\ce\ca\3f\e3")
    (data (i32.const 0x140)
         "\dd\86\00\e8\70\cb\a5\57\92\7f\00\d3\ab\57\8a\a3\43\c6\f1\d2\f3\47\0a\12\33\96\69\0b\44\4d\87\8d")
    (data (i32.const 0x160)
         "\a8\65\a9\40\f3\d1\3d\dc\a4\25\e8\ce\d2\e6\44\5e\53\c3\47\3c\d0\c5\2d\8d\f8\71\0b\39\d6\b8\fb\ae")
    (data (i32.const 0x180)
         "\bf\3c\c7\d8\b7\5e\dc\e0\93\3d\d5\fb\b6\3e\ec\39\c0\88\1e\fd\91\d1\ea\45\da\e4\c0\78\af\74\4b\cc")    (func $start (local $counter i32)

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
