# Comet Pool WASM Fixture

`comet_pool.wasm` is a byte-for-byte build of `contracts.wasm` from the Comet DEX repository (https://github.com/CometDEX/comet-contracts-v1) at commit ef4cbfad0a35202ad267c14d163d2f362995a8d3.

The tests fall back to this artifact when the `.deps/comet-contracts-v1` checkout is missing, ensuring we still exercise the real Comet swap logic without relying on additional network setup.
