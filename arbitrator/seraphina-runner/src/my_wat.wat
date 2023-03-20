(module
  (func (export "main")
    (local i32)
    i32.const 3
    local.tee 0

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
        (then return)
        (else unreachable))
  )
  (memory (export "memory") 0)
  (start 0)
)
