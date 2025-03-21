---
title: "This isn't the way to speed up Rust compile times"
date: 2023-08-27
tags:
 - rust
 - serde
 - plt
---

Recently serde, one of the most popular Rust libraries made a decision
that sped up compile times by using a precompiled version of a
procedural macro instead of compiling it on the fly. Like any
technical decision, there are tradeoffs and advantages to everything.
I don't think the inherent ecosystem risks in slinging around
precompiled binaries are worth the build speed advantages, and in this
article I'm going to cover all of the moving parts for this space.

<xeblog-hero ai="Anything v3" file="waifu-perch" prompt="1girl, green hair, green eyes, smile, hoodie, skirt, river, bridge in the distance, long hair, wearing uggs, summer, beach, space needle, crabs, -amputee"></xeblog-hero>

## serde

[serde](https://serde.rs/) is one of the biggest libraries in the Rust
ecosystem. It provides the tooling for serializing and deserializing
(ser/de) arbitrary data structures into arbitrary formats. The main
difference between serde and other approaches is that serde _doesn't
prefer an individual encoding format_. Compare this struct in Rust vs
the equivalent struct in Go:

```rust
#[derive(Debug, Deserialize, Eq, PartialEq, Clone, Serialize)]
pub struct WebMention {
    pub source: String,
    pub title: Option<String>,
}
```

```go
type WebMention struct {
    Source string  `json:"source"`
    Title  *string `json:"title"`
}
```

Besides syntax, the main difference is in how the
serialization/deserialization works. In Go the
[encoding/json](https://pkg.go.dev/encoding/json) package uses
[runtime
reflection](https://en.wikipedia.org/wiki/Reflective_programming) to
parse the structure metadata. This does work, but it's expensive
compared to having all that information already there.

The way serde works is by having an implementation of
[Deserialize](https://docs.rs/serde/latest/serde/trait.Deserialize.html)
or
[Serialize](https://docs.rs/serde/latest/serde/trait.Serialize.html)
on the data types you want to encode or decode. This effectively
pushes all of the data that is normally inspected at runtime with
reflection into compile-time data. In the process, this makes the code
run a little bit faster and more deterministically, but at the cost of
adding some time at compile time to determine that reflection data up
front.

I think this is a fair tradeoff due to the fundamental improvements in
developer experience. In Go, you have to declare the encoding/decoding
rules for every codec individually. This can lead to stuctures that
look like this:

```go
type WebMention struct {
    Source string  `json:"source" yaml:"source" toml:"source"`
    Title  *string `json:"title" yaml:"title" toml:"source"`
}
```

<xeblog-conv name="Aoi" mood="wut">Hey, in your code you have the
struct tag `toml:"source"` defined on the `Title` field, didn't you
mean to say `toml:"title"`?</xeblog-conv>
<xeblog-conv name="Cadey" mood="coffee">Good catch! The fact that you
have to declare the same thing over and over again makes it ripe for
messing things up in annoyingly trivial ways. It would be so much
better if this was all declared once. Here's the correct way to tag
this struct:</xeblog-conv>

```go
type WebMention struct {
    Source string  `json:"source" yaml:"source" toml:"source"`
    Title  *string `json:"title" yaml:"title" toml:"title"`
}
```

This becomes unwieldy and can make your code harder to read. Some
codecs get around this by reading and using the same tag rules that
encoding/json does, but the Rust equivalent works for _any_ codec that
can be serialized into or deserialized from. That same `WebMention`
struct works with JSON, YAML, TOML, [msgpack](https://msgpack.org/),
or anything else you can imagine. serde is one of the most used
packages for a reason: it's so convenient and widespread that it's
widely seen as being effectively in the standard library.

If you need to add additional behavior such as [parsing a string to
markdown](https://github.com/Xe/site/blob/f30057e759c604f7fcc700df6e1cbc6027af45f0/src/app/config/markdown_string.rs),
you can do that with your own implementation of the Deserialize trait.
I do this with the [VODs pages](/vods) in order to define my stream
VOD information in configuration. The markdown inside strings compiles
to the HTML you see [on the VOD
page](https://xeiaso.net/vods/2023/3/cursorless), including the
embedded video on [XeDN](/blog/xedn). This is incredibly valuable to
me and something I really want to keep doing until I figure out how to
switch my site to using something like
[contentlayer](https://www.contentlayer.dev/) and [MDX](https://mdxjs.com/).

<xeblog-conv name="Mara" mood="hacker" standalone>All of the VOD
information is stored in [Dhall](https://dhall-lang.org/) (read: JSON
with imports, functions, and static types) files. There's Dhall
support for serde with
[serde_dhall](https://docs.rs/serde_dhall/latest/serde_dhall/), which
this website's code uses heavily for all of the static data. This
includes [signalboost entries](/signalboost), [salary
transparency](/salary-transparency) data, [blog series](/blog/series)
metadata, and the [character information
sheets](/characters).</xeblog-conv>

## The downsides

It's not all sunshine, puppies and roses though. The main downside to
the serde approach is the fact that it relies on a procedural macro.
Procedural macros are effectively lisp-style "syntax hygenic" macros.
Effectively you can view them as a function that takes in some syntax,
does stuff to it, and then returns the result to be compiled in the
program.

This is how it can derive the serialization/deserialization code, it
takes the tokens that make up the struct type, walks through the
fields, and inserts the correct serialization or deserialization code
so that you can construct values correctly. If it doesn't know how to
deal with a given type, it will blow up at compile-time, meaning that
you may need to resort to [increasingly annoying
hacks](https://serde.rs/remote-derive.html) to get things working.

<xeblog-conv name="Cadey" mood="coffee" standalone>Pedantically, this
whole support works at the language token level, not at the type
level. You need to write wrappers around remote types in order to add
serde support because proc macros don't have access to the tokens that
make up other type definitions. You _could_ do all of this at compile
time in theory with a perfectly spherical compiler that supports
type-level metaprogramming, but the Rust compiler of today can't do
that.</xeblog-conv>

When you [write your own procedural
macro](https://doc.rust-lang.org/reference/procedural-macros.html),
you create a separate crate for this. This separate crate is compiled
against a special set of libraries that allow it to take tokens from
the Rust compiler and emit tokens back to the rust compiler. These
compiled proc macros are run as dynamic libraries inside invocations
of the Rust compiler. This means that proc macros can do _anything_ as
the permissions of the Rust compiler, including crashing the compiler,
stealing your SSH key and uploading it to a remote server, running
arbitrary commands with sudo power, and much more.

<xeblog-conv name="Mara" mood="hacker" standalone>To be fair, most
people do use this power for good. The library
[sqlx](https://github.com/launchbadge/sqlx) will allow you to check
your query syntax against an actual database to ensure that your
syntax is correct (and so they don't have to implement a compliant
parser for every dialect/subdialect of SQL). You could also envision
many different worlds where people would do behavior that sounds
suspect (such as downloading API schema from remote servers), but it
provides such a huge developer experience advantage that the tradeoff
would be worth the downsides. Everything's a tradeoff.</xeblog-conv>

## A victim of success

Procedural macros are not free. They take nonzero amounts of time to
run because they are effectively extending the compiler with arbitrary
extra behavior at runtime. This gives you a lot of power to do things
like what serde does, but as more of the ecosystem uses it more and
more, it starts taking nontrivial amounts of time for the macros to
run. This causes more and more of your build time being spent waiting
around for a proc macro to finish crunching things, and if the proc
macro isn't written cleverly enough it will potentially waste time
doing the same behavior over and over again.

This can slow down build times, which make people investigate the
problem and (rightly) blame serde for making their builds slow.
Amusingly enough, serde is used by the Rust compiler rustc and package
manager cargo. This means that the extra time compiling proc macros
are biting literally everyone, including the Rust team.

<xeblog-conv standalone name="Mara" mood="hmm">Keep in mind though
that the Rust compiler is already _very damn fast_. One of the
standard benchmarks we use across hardware is the "how fast do you
compile [xesite](https://github.com/Xe/site)" test. Xesite is a fairly
complicated Rust program that uses a bunch of crates and weird
language features like the procedural macro
[maud](https://maud.lambda.xyz) to generate HTML. If you want to run
the benchmark for yourself, install
[hyperfine](https://github.com/sharkdp/hyperfine) and run the
following command: <code><pre>hyperfine --prepare "cargo clean" "cargo
build --release"</code></pre>Here's the results on our new MacBook Pro
M2 Max:<code><pre>$ hyperfine --prepare "cargo clean" "cargo build --release"
Benchmark 1: cargo build --release
  Time (mean ± σ):     41.872 s ±  0.295 s    [User: 352.774 s, System: 22.339 s]
  Range (min … max):   41.389 s … 42.169 s    10 runs
</pre></code>In comparison, the homelab shellbox machine that
production builds are made on scores this much:<code><pre>hyperfine --prepare "cargo clean" "cargo build --release"
Benchmark 1: cargo build --release
  Time (mean ± σ):     103.852 s ±  0.654 s    [User: 1058.321 s, System: 42.296 s]
  Range (min … max):   102.272 s … 104.843 s    10
  runs</pre></code>Procedural macros are plenty fast, it's always a
tradeoff because they always could be faster. For additional timing
information about xesite builds, look at the [timing
report](https://cdn.xeiaso.net/file/christine-static/blog/2023/serde/cargo-timing-20230819T184813Z.html).</xeblog-conv>

## The change

In essence, the change makes serde's derive macro use a precompiled
binary instead of compiling a new procedural macro binary every time
you build the serde_derive dependency. This removes the need for that
macro to be compiled from source, which can speed up build times
across the entire ecosystem in a few cases.

<xeblog-conv name="Cadey" mood="coffee" standalone>To be fair, this
precompiled binary fiasco only affects x86\_84/amd64 Linux hosts. The
majority of CI runs on the planet use x86\_64 Linux hosts. Given how
much of a meme "Rust has slow compile times" has become over the last
decade, it makes sense that something had to give. It would be nice if
this affected more than _cold_ CI runs (IE: ones without a
pre-populated build cache), but I guess this is the best they can do
given the constraints of the compiler as it exists today.</xeblog-conv>

However, this means that the most commonly used crate is shipping an
arbitrary binary for production builds without any way to opt-out.
This could allow a sufficiently determined attacker to use the
serde\_derive library as a way to get code execution on _every CI
instance where Rust is used_ at the same time.

<xeblog-conv name="Aoi" mood="wut">Can't you do this anyways with a
proc macro given that it's a dynamic library in the
compiler?</xeblog-conv>
<xeblog-conv name="Cadey" mood="coffee">Well, yeah, sure. The main
difficulty is that when you're doing it in a proc macro you have to
have the code in a human-readable format somewhere along the line.
This would allow users to discover that the version of the code
distributed with the crate differs from the version inside source
control fairly trivially. Compare this to what you'd have to do in
order to determine if a binary is compiled from different source code.
That requires a completely different set of skills than comparing
source code.<br/><br/>
Combine that with the fact that the Rust ecosystem doesn't currently
have a solid story around cryptographic signatures for crates and you
get a pretty terrible situation all around.<br /><br /><xeblog-picture
path="blog/2023/serde/gpg-ux"></xeblog-picture></xeblog-conv>

But this does speed things up for everyone...at the cost of using serde
as a weapon to force ecosystem change.

In my testing the binary they ship is a statically linked Linux
binary:

```
$ file ./serde_derive-x86_64-unknown-linux-gnu
./serde_derive-x86_64-unknown-linux-gnu: ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), static-pie linked, BuildID[sha1]=b8794565e3bf04d9d58ee87843e47b039595c1ff, stripped

$ ldd ./serde_derive-x86_64-unknown-linux-gnu
        statically linked
```

<xeblog-conv name="Mara" mood="hacker">Note: you should [never run
`ldd` on untrusted
executables](https://catonmat.net/ldd-arbitrary-code-execution). `ldd`
works by setting the environment variable `LD_TRACE_LOADED_OBJECTS=1`
and then executing the command. This causes your system's C dynamic
linker/loader to print all of the dependencies, however malicious
applications can and will still execute their malicious code even when
that environment variable is set. I've seen evidence of applications
exhibiting different malicious behavior when that variable is set.
Stay safe and use virtual machines when dealing with unknown
code.</xeblog-conv>
<xeblog-conv name="Numa" mood="delet">To [misquote James
Mickens](https://scholar.harvard.edu/files/mickens/files/towashitallaway.pdf),
the best way to find out what something does is by executing it to
discover more clues.</xeblog-conv>

Frustratingly, a friend of mine that uses
[cargo2nix](https://github.com/cargo2nix/cargo2nix) is reporting
getting a "file not found" error when trying to build programs
depending on serde. This is esepecially confusing given that the
binary is a statically linked binary, but I guess we'll figure out
what's going on in the future.

<xeblog-conv name="Aoi" mood="wut">Wait, but if the proc macro binary
exists how could the file not be found?</xeblog-conv>
<xeblog-conv name="Mara" mood="hacker">That's the fun part. That error
message doesn't just show up when you ask the computer to run a binary
that doesn't exist. It also shows up when the binary is loading and
the kernel is loading the dynamically linked dependencies. So the
program binary can exist but if a dynamic dependecy doesn't, it'll
bail and fail like that.</xeblog-conv>
<xeblog-conv name="Cadey" mood="coffee">Yeeep, this is one of the
worst errors in the Linux ecosystem. Don't feel bad about it being
confusing, this bites _everyone_ eventually. The first time I
encountered it, I spent more time than I'm comfortable admitting
figuring it out. I had to resort to using strace. I felt like a
massive idiot when I figured it out.</xeblog-conv>

There's also additional concerns around [the binary in question not
being totally
reproducible](https://github.com/serde-rs/serde/issues/2538#issuecomment-1684117378),
which is slightly concerning from a security standpoint. If we're
going to be trusting some random guy's binaries, I think we are in the
right to demand that it is byte-for-byte reproducible on commodity
hardware without having to reverse-engineer the build process and
figure out which _nightly version of the compiler_ is being used to
compile this binary blob that will be run everywhere.

I also can't imagine that distribution maintainers are happy with this
now that Rust is basically required to be in distribution package
managers. It's unfortunate to see [crates.io](https://crates.io) turn
from a source code package manager to a binary package manager like
this.

<xeblog-conv name="Numa" mood="delet">Nah, trust me bro. It's totes a
legit binary, don't think about it so much and just run this arbitrary
code on your system. What could go wrong?</xeblog-conv>
<xeblog-conv name="Aoi" mood="coffee">Uhhhh, a lot??? Especially if
this becomes a common practice that is validated by the biggest
project using it. This feels like it could have a massive chilling
effect across the entire ecosystem where this behavior becomes more
normalized and expected. I don't know if I'd want to see that become a
reality.</xeblog-conv>

### This doesn't even make build times faster

The most frustrating part about this whole affair is that while I was
writing the majority of this article, I assumed that it actually sped
up compliation. Guess what: it only speeds up compilation when you
are doing a brand new build without an existing build cache. In many
cases this means that you only gain the increased build speed in very
limited cases: when you are doing a brand new clean build or when you
update serde_derive.

<xeblog-conv name="Aoi" mood="wut" standalone>I guess these are some
semi-common usecases where this would be useful, but I don't think
this is worth the extra threat vector.</xeblog-conv>

This would be much more worth the tradeoff if it actually gave a
significant compile speed tradeoff, but in order for this to make
sense you'd need to be building many copies of serde\_derive in your
CI builds constantly. Or you'd need to have every procedural macro in
the ecosystem also follow this approach. Even then, you'd probably
only save about 20-30 seconds in cold builds on extreme cases. I
really don't think it's worth it.

## The middle path

Everything sucks here. This is a Kobayashi Maru situation. In order to
really obviate the need for these precompiled binary blobs being used
to sidestep compile time you'd need a complete redesign of the
procedural macro system.

<xeblog-conv name="Cadey" mood="angy" standalone>Or, you'd need the
proper compile-time reflection support that
[ThePHD](https://thephd.dev/) was going to work on until the whole
RustConf debacle happened. This would entirely obviate the need for
the derive macro serde uses in its current form. We could have had
nice things.</xeblog-conv>

One of the huge advantages of the proc macro system as it currently
exists is that you can easily use any Rust library you want at compile
time. This makes doing things like generating C library bindings on
the fly using [`bindgen`](https://rust-lang.github.io/rust-bindgen/)
trivial.

<xeblog-conv name="Aoi" mood="wut">How does that work though? It can't
do something awful like parsing the C/C++ headers manually, can
it?</xeblog-conv>
<xeblog-conv name="Numa" mood="happy">That's the neat part, it
actually does do that by using [clang](https://clang.llvm.org/)'s
C/C++ parser!</xeblog-conv>
<xeblog-conv name="Aoi" mood="coffee">That's horrifying.</xeblog-conv>
<xeblog-conv name="Mara" mood="hacker">It is yeah, but this is what
you have to do in the real world to get things working. It's worth
noting that you don't have to always do this at compile time. You can
commit the intermediate code to your git repo or [write your bindings
by
hand](https://github.com/tailscale/pam/blob/main/cmd/pam_tailscale/src/pam.rs),
but I think it's better to take the build speed loss and have things
get generated for you so you can't forget to do it.</xeblog-conv>

Maybe there could be a lot of speed to be gained with aggressive
caching of derived compiler code. I think that could solve a lot of
the issues at the cost of extra disk space being used. Disk space is
plenty cheap though, definitely cheaper than developer time. The
really cool advantage of making it at the derive macro level is that
it would also apply for traits like
[Debug](https://doc.rust-lang.org/std/fmt/trait.Debug.html) and
[Clone](https://doc.rust-lang.org/std/clone/trait.Clone.html) that are
commonly derived anyways.

I have no idea what the complexities and caveates of doing this would
be, but it could also be interesting to have the crate publishing step
do aggressive borrow checking logic for every supported platform but
then disable the borrow checker on crates downloaded from crates.io.
The borrow checker contributes a lot of time to the compilation
process, and if you gate acceptance to crates.io on the borrow checker
passing then you can get away without needing to run the extra borrow
checker logic when compiling dependencies.

<xeblog-conv name="Aoi" mood="wut">Yeah but when the borrow checker
changes behavior slightly within the same Rust edition, what happens?
What if there is a bug that allows something to pass muster in one
version of rustc that shouldn't be allowed, making the code in
crates.io fundamentally wrong?</xeblog-conv>
<xeblog-conv name="Cadey" mood="coffee">I claimed ignorance of the
problems for a reason! I realize that this would nearly impossible in
practice, but I feel like this could be more of a viable option than
telling people it's okay to put binaries in the mostly source-code
based package store that is
[crates.io](https://crates.io).</xeblog-conv>

<details>
<summary>Tangent about using WebAssembly</summary>

### WASM for procedural macros?

<xeblog-conv name="Aoi" mood="wut">Wait, how is this relevant here?
This seems like a nonsequitor, doing proc macro compliation/running
with WebAssembly would undoubtedly be slower, right? If only going by
the rule that a layer of abstraction is by definition more overhead
than not having it?</xeblog-conv>
<xeblog-conv name="Cadey" mood="coffee">The maintainer of serde is
also the creator of [watt](https://github.com/dtolnay/watt), a runtime
for executing precompiled procedural macros with WebAssembly. Adopting
a solution like this would vastly improve the security, isolation, and
reproducibility of procedural macros. I really wish this was more
widespread. With optimizations such as adopting
[wasmtime](https://wasmtime.dev/) for executing these proc macros, it
could be made a lot faster on standard development/production
environments while also not leaving people on obscure targets like
rv64-gc in the dust.<br /><br />I'm also pretty sure that there is an
easier argument to be made for shipping easily replicatable WASM blobs
[like Zig does](https://ziglang.org/news/goodbye-cpp/) instead of
shipping around machine code like serde does.</xeblog-conv>

One of the core issues with procedural macros is that they run
unsandboxed machine code. Sandboxing programs is basically impossible
to do cross-platform without a bunch of ugly hacks at every level. 

I guess you'd need to totally rewrite the proc macro system to use
[WebAssembly](https://webassembly.org/) instead of native machine
code. Doing this with WebAssembly would let the Rust compiler control
the runtime environment that applications would run under. This would
let packages do things like:

* Declare what permissions it needs and have permissions changes on
  updates to the macros cause users to have to confirm them
* Declare "cache storage" so that things like derive macro
  implementations could avoid needing to recompute code that has
  already passed muster.
* Let people ship precompiled binaries without having to worry as much
  about supporting every platform under the sun. The same binary would
  run perfectly on every platform.
* More easily prove reproducibility of the proc macro binaries,
  especially if the binaries were built on the crates.io registry
  server somehow.
* Individually allow/deny execution of commands so that common
  behaviors like `bindgen`, `pkg-config`, and compiling embedded C
  source code continue working.
  
This would require _a lot_ of work and would probably break a lot of
existing proc macro behavior unless care was taken to make things as
compatible. One of the main pain points would be dealing with C
dependencies as it is nearly impossible\* to deterministically prove
where the dependencies in question are located without running a bunch
of shell script and C code.

<xeblog-conv standalone name="Cadey" mood="coffee">\*If you are using
Nix, this is trivial, but sadly we aren't at a place where Nix is used
by everyone yet.</xeblog-conv>

One of the biggest headaches would be making a WebAssembly JIT/VM that
would work well enough across platforms that the security benefits
would make up for the slight loss in execution speed. This is
annoyingly hard to sell given that the current state of the world is
suffering from long compilation times. It also doesn't help that
WebAssembly is still very relatively new so there's not yet the level
of maturity needed to make things stable. There is a POSIX-like layer
for WebAssembly programs called [WASI](https://wasi.dev) that does
bridge a lot of the gap, but it misses a lot of other things that
would be needed for full compatibility including network socket and
subprocess execution support.

<xeblog-conv name="Mara" mood="happy" standalone>There is an extension
to WASI called [WASIX](https://wasix.org/) that does solve nearly all
of the compatibility problems, but WASIX isn't standard yet and my
runtime of choice [wazero](https://wazero.io/) doesn't have out-of-the
box support for it yet. Hopefully [it will be supported
soon](https://github.com/tetratelabs/wazero/issues/1495)!</xeblog-conv>



</details>

---

This entire situation sucks. I really wish things were better.
Hopefully the fixes in
[serde-rs/serde#2580](https://github.com/serde-rs/serde/pull/2580)
will be adopted and make this entire thing a non-issue. I understand
why the serde team is making the decisions they are, but I just keep
thinking that this isn't the way to speed up Rust compile times. There
has to be other options.

I don't know why they made serde a malware vector by adding this
unconditional precompiled binary in a patch release in exchange for
making cold builds in CI barely faster.

The biggest fear I have is that this practice becomes widespread
across the Rust ecosystem. I really hate that the Rust ecosystem seems
to have so much drama. It's scaring people away from using the tool to
build scalable and stable systems. Until there's closure on this, I'll just
keep writing [my hobby code in Go](https://github.com/Xe/x). I really
hope I don't have to port my website back to Go due to another spat of
community drama targeting one of the libraries I depend on (eg: axum,
rustls). That would suck, but I could deal with it if I had to.

<xeblog-conv name="Cadey" mood="percussive-maintenance">I mean at some
level, to be in a community is to eventually cause conflict. I'm not
tired of the conflicts existing, I'm tired of the conflicts being
poorly handled and spilling out into GitHub hellthreads that leave
everyone unhappy. Let's hope this event doesn't spill out into even
more intelligent and highly capable people burning out and
leaving.</xeblog-conv>
