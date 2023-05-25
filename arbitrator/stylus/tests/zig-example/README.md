# Zig example

Install [Zig](https://ziglang.org/learn/getting-started/)

Then, navigate to `cd arbitrator/stylus/tests/zig-example/src` and run

```
zig build-lib lib.zig -target wasm32-freestanding -dynamic --export=arbitrum_main
```

Next, test it out in `arbitrator/stylus/src/test/native.rs` under `test_zig`
