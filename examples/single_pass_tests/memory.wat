(module
    (memory 1)
    (func $main (export "main")
        (call $test_stack_layout)
    )

    (func $test_stack_layout
        (local $addr i32)
        (set_local $addr (i32.const 16))

        (i32.store (get_local $addr) (i32.const 10))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 655360))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 11))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 720896))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 12))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 786432))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 13))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 851968))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 14))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 917504))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 15))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 983040))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 16))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 1048576))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 17))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 1114112))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 18))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 1179648))
            (then)
            (else (unreachable))
        )

        (i32.const 1)
        (i32.store (get_local $addr) (i32.const 19))
        (if (i32.eq (i32.load (i32.const 14)) (i32.const 1245184))
            (then)
            (else (unreachable))
        )

        (drop)
        (drop)
        (drop)
        (drop)
        (drop)
        (drop)
        (drop)
        (drop)
        (drop)
    )
)