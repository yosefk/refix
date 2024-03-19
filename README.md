# Replacing fixed-sized string prefixes in binaries to refix them to their build context

Here's [the long story](https://yosefk.com/blog/refix-fast-debuggable-reproducible-builds.html) about what `refix` does and why you'd want to do this.

The short story is, `refix` **re**places **fix**ed-size string p**refix**es inside binary files (executables, shared libraries, static libraries, and object files.) It can also replace
the data of whole sections with equally sized data read from a file (if `--section name file` is passed one or more times.)

Why would you want this? Let's say you have reproducible builds, which have relative paths to source code, and no build-specific information
(like the absolute path of the final executable, the build time etc.) Then you might want - I'd say you *very much do want* - to put
the absolute source code paths, and all that build-specific information, into the executable once it leaves the build system cache
(where only reproducible artifacts belong) and is delivered to your filesystem. You want to _refix_ the binary back to the original
source path & other build context, as it were, after the build system detached it from this context.

Putting this info back helps debugging a lot!
You don't want finding the source code in debuggers and other tools to be a puzzle; you want it to just work.

# Putting absolute source file paths into your binaries

* Run with the equivalent of the following `gcc` flags:
  <pre>
  -fdebug-prefix-map==MAGIC <i># for DWARF</i>
  -ffile-prefix-map==MAGIC  <i># for __FILE__</i>
  </pre>
* Make MAGIC long enough for any source path prefix you're willing to support.
* Why the `==` in the flag? This invocation assumes that file paths
    are relative, so it remaps _the empty string_ to MAGIC, meaning, `dir/file.c` becomes `MAGICdir/file.c`.
    You can also pass `=/prefix/to/remap=MAGIC`, if your build system uses absolute paths.
* After the linked binary is delivered, use `refix` to put the actual source path back in (this basically works
  like replacing with `sed` would, but taking tens of milliseconds instead of seconds for large binaries):
  <pre>refix binary MAGIC actual-source-prefix</pre>
* If the source path is shorter than the length of MAGIC, pad it with forward slashes: `/////home/user/src/`.
  If the source path is too long, the post-link step should truncate it, warn, and eventually be changed to outright fail.

That's it, now all debugging tools will find the source code effortlessly!

# Putting other build-specific info into your binaries

You can put all the "build context" info (full path to the executable, build date etc.) into a separate section,
reserved at build time and filled after link time. You make the section with:

```c++
char ver[SIZE] __attribute__((section(".ver"))) = {1};
```

This reserves `SIZE` bytes in a section called `.ver`. It's non-`const` deliberately, since
if it's `const`, the OS will exclude it from core dumps (why save data to disk when it's guaranteed
to be exactly the same as the contents of the section in the binary?) But you might actually very much want to look at
the content of this section in a core dump, perhaps before looking at anything else.
For instance, looking at this section is how you can find the path of the executable that dumped this core!

How do you find the section in the core dump without having an executable which the debugger could use
 to tell you the address of `ver`? Like so: `strings core | grep MagicOnlyFoundInVer`. What if this section,
 being non-`const`, gets overwritten? If you're really worried about this, you can align its base address
 and size to the OS page size, and `mprotect` it at init time, though I personally never bothered and have
 not suffered any consequences to date.

Additionally, our `ver` variable is deliberately initialized with one `1` followed by zeros, since if it's
all zeros, then `.ver` will be a "bss" section, the kind zeroed by the loader and
without space reserved for it in the binary. So you'd have nowhere to write your
actual, "non-reproducible" version info at a post-link step.

After the linker is done, and you're running `refix` to fix the source path, you can pass it more arguments to replace the section:

```
refix binary MAGIC actual-source-prefix --section .ver file
```

`refix` will put the content of `file` into `.ver`, or fail if the file's length differs from the section's.
You could do this with `objcopy`, same as you could replace the prefixes with `sed`; the point of `refix`
is doing it faster (by optimizing for the "same-sized old & new string length" case, as well as knowing
to ignore most of the input file where nothing ever needs to be changed.)

Of course, you needn't have this "version info section" to use refix for putting the source path
into your binaries. It's another, separate thing you can do that helps with debugging.
