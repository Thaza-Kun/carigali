# Preprocessor: Wikipedia

Preprocesses wikidumps into markdown-like files.

Use [`wikimedia-rs`](https://github.com/fluffysquirrels/wikimedia-rs) to download the desired wikidump Use WSL if you are in windows. It cannot be compiled on Windows.

- `src/main.rs` breaks down the large xml into small markdown-like text files.
- `xtask/src/main.rs` runs a bun server and converts all the wikimedia markups into clean text with minimal markup.

> [!NOTE]
> We rely on the JS library [`wtf-wikipedia`](https://github.com/spencermountain/wtf_wikipedia) to parse the convoluted wikimedia markup.