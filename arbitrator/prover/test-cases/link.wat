;; Copyright 2023, Offchain Labs, Inc.
;; For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

(module
    (import "hostio" "wavm_link_module" (func $link (param i32) (result i32)))
    (import "hostio" "wavm_unlink_module" (func $unlink (param) (result)))
    (data (i32.const 0x000)
         "\2e\fa\1a\aa\e7\14\f2\63\bc\a1\c3\76\8a\46\e9\04\8a\a5\e7\c0\2d\fe\86\cc\30\0d\7e\de\18\1e\5a\46")
    (data (i32.const 0x020)
         "\1f\cd\e2\81\95\13\e0\4d\c5\c2\ce\f7\63\e7\18\10\db\f8\b2\eb\56\3f\37\b1\35\f6\61\67\02\91\7a\9d")
    (data (i32.const 0x040)
         "\6f\5f\86\5e\fd\50\47\4a\71\22\bf\7f\e0\25\80\e1\2d\58\2c\3c\a5\f3\68\20\70\f4\c1\af\cd\c5\d5\1a")
    (data (i32.const 0x060)
         "\7f\58\de\0d\bc\ed\d2\ef\5e\b0\f8\37\6c\0b\84\0f\52\86\4d\b6\08\87\be\be\44\b7\7a\e4\a4\24\e2\de")
    (data (i32.const 0x080)
         "\4e\16\85\9c\b7\f3\ed\8d\e3\98\30\36\5a\ef\c9\aa\c6\31\fb\1e\39\54\a5\bd\09\9a\01\a5\6d\03\d6\17")
    (data (i32.const 0x0a0)
         "\4d\88\c3\ad\5b\31\5e\96\1c\79\cb\44\ee\f4\b9\42\18\da\fc\25\c1\26\fd\a5\1e\d4\b5\30\6c\59\38\ab")
    (data (i32.const 0x0c0)
         "\ac\23\98\89\12\fc\97\81\88\66\e8\19\dc\f6\f5\33\9d\75\bf\ae\6a\b2\0c\2a\5a\b4\12\87\29\1f\e2\d1")
    (data (i32.const 0x0e0)
         "\79\ab\25\ae\44\5f\37\d5\e4\4f\2f\b8\9f\43\c7\ba\4c\c0\66\3b\97\e9\ab\0c\88\a0\ba\01\02\3e\b7\a5")
    (data (i32.const 0x100)
         "\dd\34\a3\48\d3\a0\b3\05\bc\c1\b7\de\fb\87\ec\00\d7\43\26\5e\6b\22\85\26\a2\77\35\e1\a2\ca\3c\79")
    (data (i32.const 0x120)
         "\6f\d2\78\74\f4\a5\3b\84\1f\69\5c\80\5e\7c\12\52\ee\50\92\c2\f7\4f\ba\ab\35\51\84\53\fd\90\a8\11")
    (data (i32.const 0x140)
         "\cb\ff\fb\39\a0\f9\d4\3e\4a\46\a4\67\68\37\09\5e\d7\4e\3e\c9\76\da\60\fc\d9\44\93\5d\ef\8d\6d\0c")
    (data (i32.const 0x160)
         "\c6\62\35\4e\f0\98\48\b7\af\03\5f\8d\09\92\8f\bf\cc\7d\71\1c\3e\88\19\11\77\6b\cd\c4\20\91\4c\3c")
    (data (i32.const 0x180)
         "\c6\50\44\e7\ff\43\22\65\1c\19\65\48\57\0a\40\c7\20\62\2c\b2\0e\3a\71\cf\a7\05\2c\f2\6f\dc\5e\06")
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
