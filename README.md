# Indexed-db Panic

This is a little app that causes indexed-db to panic.  Its sole
purpose is to [provide an
example](https://github.com/ctm/mb2-doc/issues/1636) for the
indexed-db maintainers. However, if you're curious about its origin,
[this
comment](https://github.com/ctm/mb2-doc/issues/1632#issuecomment-3039775943)
provides a tiny bit of context.

This app started as a fairly minimal template for a Yew app that's
built with [Trunk]. I then hacked in the portion of mb2's (mb2 is
closed-source) CSS upload code that caused the panic. After that, I
stripped out as much as I could while still getting the panic.

## Usage

For a more thorough explanation of Trunk and its features, please head over to the [repository][trunk].

### Installation

If you don't already have it installed, it's time to install Rust: <https://www.rust-lang.org/tools/install>.
The rest of this guide assumes a typical Rust installation which contains both `rustup` and Cargo.

To compile Rust to WASM, we need to have the `wasm32-unknown-unknown` target installed.
If you don't already have it, install it with the following command:

```bash
rustup target add wasm32-unknown-unknown
```

Now that we have our basics covered, it's time to install the star of the show: [Trunk].
Simply run the following command to install it:

```bash
cargo install trunk wasm-bindgen-cli
```

That's it, we're done!

### Running

```bash
trunk serve --open
```

Rebuilds the app whenever a change is detected and runs a local server to host it.

Once it's running, click on the `Choose File` button to "upload" a
file to your Browser (i.e., this is client-side only upload; you're
not sending anything to a server).  Then&mdash;after uploading a
file&mdash;restart the app by refreshing the page. In the JavaScript
console, you should see a `Transaction blocked without any request
under way` panic followed by a back-trace (at least until the
indexed-db bug has been fixed).

### License

The template ships with both the Apache and MIT license. I've removed the Apache
license and added the Unlicense.

[trunk]: https://github.com/thedodd/trunk
