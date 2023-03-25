(module
    (func $main (export "main") (param $loops i32) (result i64) 
    (local $mul i64)
    (local $inc i64)
    (local $state i64)
    ;; initialize the generator
    i64.const 6364136223846793005
    local.set $mul 
    i64.const 1442695040888963407
    local.tee $inc 
    i64.const 14627392581883831781
    i64.add ;; initial state is state + inc for some reason
    local.set $state 
    ;; advance the generator $loops times 
    (loop $loop 
        ;; multiply state by mul and add inc
        local.get $state 

        local.get $mul 
        i64.mul 

        local.get $inc 
        i64.add 

        local.set $state 

        ;; subtract 1 from loop count and exit loop if zero 
        local.get $loops 
        i32.const 1 
        i32.sub 
        local.tee $loops 
        i32.const 0 
        i32.ne   
        br_if $loop 
    )
    local.get $state 
    return 
    )
    (memory (export "memory") 0)
)