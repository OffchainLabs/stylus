(module
  (func (export "main") 
  (i32.const 1)
  (i32.const 2)
  (call $do_work)
  )
  (func $do_work (export "do_work") (param i32 i32)
    ;; (local i32 i32)
    i32.const 3
    local.tee 0
    local.tee 1 
    
    i32.const 2 
    i32.mul 
    
    i32.const 2 
    i32.mul 
    
    i32.const 2 
    i32.mul 

    local.get 0
    i32.add 

    ;; should have 3 * 2^3 + 3 = 27 on stack
    i32.const 27 
    i32.eq 
    (if 
        (then nop)
        (else unreachable))
    i32.const 5
    i32.const 5
    i32.const 5
    return
  )

  (memory (export "memory") 0)
  (start 0)
)
