# SV2 Integration Tests

This is a test crate and it can be used in order to test the behavior of different roles when
working together. Each role should have a `start_[role_name]` function under `common` folder that
can be called in order to run the role. In order to assert the behavior of the role or the messages
it exchanges with other roles, you can use the `Sniffer` helper in order to listen to the messages
exchanged between the roles, and assert those messages using the `assert_message_[message_type]`
function. For examples on how to use the `Sniffer` helper, you can check the
`sniffer_integration.rs` module or other tests in the `tests` folder.

All our tests run in either regtest or signet network. We download Bitcoin Core v30 binaries
from https://bitcoincore.org/bin/bitcoin-core-30.0/ and the Template provider (sv2-tp) binaries
from https://github.com/stratum-mining/sv2-tp/releases. Bitcoin Core runs via IPC, and sv2-tp
provides Stratum V2 template distribution. These are the only external dependencies in our tests.

## Running Instructions

In order to run the integration tests, you can use the following command:

```bash
$ git clone git@github.com:stratum-mining/stratum.git
$ cargo test --manifest-path=integration-tests/Cargo.toml --verbose --test '*' -- --nocapture
```

Note: during the execution of the tests, the `template-provider` directory holds the downloaded
binaries (Bitcoin Core and sv2-tp), while test data directories are created in the system temp
directory.

## License
MIT OR Apache-2.0
